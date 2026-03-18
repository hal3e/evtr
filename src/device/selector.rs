use std::{
    fs, io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use crossterm::event::{
    Event, EventStream as TermEventStream, KeyCode, KeyEventKind, KeyModifiers,
};
use evdev::Device;
use futures::StreamExt;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Style, Stylize, palette::tailwind},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Widget, Wrap},
};

use super::State;
use crate::{
    device::popup::{Popup, render_popup},
    error::{Error, ErrorArea, Result},
};

const TEXT_COLOR: ratatui::style::Color = tailwind::SLATE.c200;
const PAGE_SCROLL_SIZE: usize = 10;
const LAYOUT_MARGIN_PCT: u16 = 20;
const LAYOUT_CONTENT_WIDTH_PCT: u16 = 60;
const TOP_PADDING_HEIGHT: u16 = 1;
const SEARCH_BOX_HEIGHT: u16 = 3;
const MAIN_MIN_HEIGHT: u16 = 3;
const POPUP_MIN_WIDTH: u16 = 10;
const POPUP_MIN_HEIGHT: u16 = 3;
const POPUP_MAX_WIDTH: u16 = 80;
const HELP_POPUP_MIN_WIDTH: u16 = 30;
const HELP_POPUP_MIN_HEIGHT: u16 = 6;
const HELP_POPUP_MAX_WIDTH: u16 = 80;
const INPUT_DIR: &str = "/dev/input";
const INPUT_EVENT_PREFIX: &[u8] = b"event";

#[derive(Debug)]
pub struct DeviceInfo {
    pub device: Device,
    pub identifier: String,
}

#[derive(Debug)]
struct DiscoveryError {
    path: PathBuf,
    kind: io::ErrorKind,
    message: String,
}

impl DiscoveryError {
    fn new(path: impl Into<PathBuf>, err: io::Error) -> Self {
        Self {
            path: path.into(),
            kind: err.kind(),
            message: err.to_string(),
        }
    }
}

#[derive(Debug)]
struct DiscoveryResult<T> {
    devices: Vec<T>,
    event_nodes: usize,
    skipped: usize,
    had_read_dir_error: bool,
    first_read_dir_error: Option<DiscoveryError>,
    first_open_error: Option<DiscoveryError>,
}

impl<T> DiscoveryResult<T> {
    fn new() -> Self {
        Self {
            devices: Vec::new(),
            event_nodes: 0,
            skipped: 0,
            had_read_dir_error: false,
            first_read_dir_error: None,
            first_open_error: None,
        }
    }

    fn read_dir_failed(path: impl Into<PathBuf>, err: io::Error) -> Self {
        let mut result = Self::new();
        result.record_read_dir_error(path, err);
        result
    }

    fn record_read_dir_error(&mut self, path: impl Into<PathBuf>, err: io::Error) {
        self.had_read_dir_error = true;
        if self.first_read_dir_error.is_none() {
            self.first_read_dir_error = Some(DiscoveryError::new(path, err));
        }
    }

    fn record_open_error(&mut self, path: impl Into<PathBuf>, err: io::Error) {
        self.skipped += 1;
        if self.first_open_error.is_none() {
            self.first_open_error = Some(DiscoveryError::new(path, err));
        }
    }

    fn issue(&self) -> Option<DiscoveryIssue> {
        if !self.devices.is_empty() {
            return None;
        }

        if let Some(error) = &self.first_read_dir_error {
            return Some(DiscoveryIssue::ReadDir {
                path: error.path.clone(),
                message: error.message.clone(),
            });
        }

        if self.event_nodes == 0 {
            return Some(DiscoveryIssue::NoDevicesFound);
        }

        if let Some(error) = &self.first_open_error {
            if error.kind == io::ErrorKind::PermissionDenied {
                return Some(DiscoveryIssue::PermissionDenied {
                    skipped: self.skipped,
                });
            }

            return Some(DiscoveryIssue::OpenFailed {
                skipped: self.skipped,
                path: error.path.clone(),
                message: error.message.clone(),
            });
        }

        Some(DiscoveryIssue::NoDevicesFound)
    }

    fn error_message(&self) -> Option<String> {
        self.issue().map(|issue| issue.message())
    }
}

#[derive(Debug, PartialEq, Eq)]
enum DiscoveryIssue {
    ReadDir {
        path: PathBuf,
        message: String,
    },
    PermissionDenied {
        skipped: usize,
    },
    OpenFailed {
        skipped: usize,
        path: PathBuf,
        message: String,
    },
    NoDevicesFound,
}

impl DiscoveryIssue {
    fn message(&self) -> String {
        match self {
            DiscoveryIssue::ReadDir { path, message } => {
                format!("unable to read {}: {}", path.display(), message)
            }
            DiscoveryIssue::PermissionDenied { skipped } => {
                format!(
                    "found {skipped} input device node(s), but none were readable; check permissions for /dev/input/event*"
                )
            }
            DiscoveryIssue::OpenFailed {
                skipped,
                path,
                message,
            } => {
                format!(
                    "found {skipped} input device node(s), but none could be opened; first error: {}: {}",
                    path.display(),
                    message
                )
            }
            DiscoveryIssue::NoDevicesFound => Error::NoDevicesFound.to_string(),
        }
    }
}

fn filtered_indexes_by_query<T, F>(
    items: &[T],
    query: &str,
    matcher: &SkimMatcherV2,
    identifier_of: F,
) -> Vec<usize>
where
    F: Fn(&T) -> &str,
{
    if query.is_empty() {
        return (0..items.len()).collect();
    }

    let mut scored_items: Vec<(usize, i64)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matcher
                .fuzzy_match(identifier_of(item), query)
                .map(|score| (index, score))
        })
        .collect();

    scored_items.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    scored_items.into_iter().map(|(index, _)| index).collect()
}

pub struct DeviceSelector {
    devices: Vec<DeviceInfo>,
    filtered_indexes: Vec<usize>,
    selected_filtered_index: usize,
    search_query: String,
    matcher: SkimMatcherV2,
    error_message: Option<String>,
    help_visible: bool,
    help_lines: Vec<String>,
}

impl DeviceSelector {
    fn new(error_message: Option<String>) -> Self {
        let discovery = Self::discover_devices();
        let discovery_message = discovery.error_message();
        let devices = discovery.devices;
        let filtered_indexes = (0..devices.len()).collect();

        Self {
            devices,
            filtered_indexes,
            selected_filtered_index: 0,
            search_query: String::new(),
            matcher: SkimMatcherV2::default(),
            error_message: error_message.or(discovery_message),
            help_visible: false,
            help_lines: Self::help_lines(),
        }
    }

    fn discover_devices() -> DiscoveryResult<DeviceInfo> {
        let entries = match fs::read_dir(INPUT_DIR) {
            Ok(entries) => entries,
            Err(err) => return DiscoveryResult::read_dir_failed(INPUT_DIR, err),
        };

        let mut result = Self::discover_from_entries(
            entries.map(|entry| entry.map(|entry| entry.path())),
            Self::open_device,
        );
        result.devices.sort_unstable_by(|a, b| {
            a.identifier
                .to_lowercase()
                .cmp(&b.identifier.to_lowercase())
        });

        result
    }

    fn discover_from_entries<T, I, F>(entries: I, mut open_device: F) -> DiscoveryResult<T>
    where
        I: IntoIterator<Item = io::Result<PathBuf>>,
        F: FnMut(&Path) -> io::Result<T>,
    {
        let mut result = DiscoveryResult::new();

        for entry in entries {
            match entry {
                Ok(path) => {
                    if !Self::is_event_node(&path) {
                        continue;
                    }

                    result.event_nodes += 1;
                    match open_device(&path) {
                        Ok(device) => result.devices.push(device),
                        Err(err) => result.record_open_error(path, err),
                    }
                }
                Err(err) => result.record_read_dir_error(INPUT_DIR, err),
            }
        }

        result
    }

    fn is_event_node(path: &Path) -> bool {
        let Some(name) = path.file_name() else {
            return false;
        };

        name.as_bytes().starts_with(INPUT_EVENT_PREFIX)
    }

    fn open_device(path: &Path) -> io::Result<DeviceInfo> {
        let device = Device::open(path)?;
        let name = device.name().unwrap_or("Unknown Device").to_string();
        let path = path.to_string_lossy().to_string();

        Ok(DeviceInfo {
            device,
            identifier: format!("{name} ({path})"),
        })
    }

    fn apply_discovery(&mut self, discovery: DiscoveryResult<DeviceInfo>) {
        let error_message = discovery.error_message();
        self.devices = discovery.devices;
        self.update_filter();
        self.error_message = error_message;
    }

    fn refresh_devices(&mut self) {
        self.apply_discovery(Self::discover_devices());
    }

    pub async fn run(
        terminal: &mut DefaultTerminal,
        error_message: Option<String>,
    ) -> Result<State> {
        let mut selector = Self::new(error_message);
        let mut term_events = TermEventStream::new();

        loop {
            terminal
                .draw(|frame| {
                    selector.render(frame.area(), frame.buffer_mut());
                })
                .map_err(|err| Error::io(ErrorArea::Selector, "selector draw", err))?;

            match term_events.next().await {
                Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                    if selector.help_visible {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') => {
                                selector.help_visible = false;
                            }
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                return Ok(State::Exit);
                            }
                            _ => {}
                        }
                        continue;
                    }
                    if selector.error_message.is_some() {
                        selector.error_message = None;
                    }
                    match key.code {
                        KeyCode::Char('?') => {
                            selector.toggle_help();
                        }
                        KeyCode::Enter => {
                            if selector.filtered_indexes.is_empty() {
                                selector.refresh_devices();
                                continue;
                            }
                            if let Some(&index) = selector
                                .filtered_indexes
                                .get(selector.selected_filtered_index)
                            {
                                return Ok(State::Monitor(Box::new(
                                    selector.devices.swap_remove(index),
                                )));
                            }
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(State::Exit);
                        }
                        KeyCode::Esc => {
                            if selector.search_query.is_empty() {
                                return Ok(State::Exit);
                            } else {
                                selector.clear_search();
                            }
                        }
                        KeyCode::Up => {
                            selector.navigate_up();
                        }
                        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            selector.navigate_up();
                        }
                        KeyCode::Down => {
                            selector.navigate_down();
                        }
                        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            selector.navigate_down();
                        }
                        KeyCode::PageUp => {
                            selector.navigate_page(-1);
                        }
                        KeyCode::PageDown => {
                            selector.navigate_page(1);
                        }
                        KeyCode::Home => selector.select_home(),
                        KeyCode::End => selector.select_end(),
                        KeyCode::Backspace => {
                            selector.remove_char();
                        }
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            selector.clear_search();
                        }
                        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            selector.refresh_devices();
                        }
                        KeyCode::Char(c)
                            if key.modifiers == KeyModifiers::SHIFT || key.modifiers.is_empty() =>
                        {
                            selector.add_char(c);
                        }
                        _ => {}
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(err)) => {
                    return Err(Error::io(ErrorArea::Selector, "terminal event stream", err));
                }
                None => {
                    return Err(Error::stream_ended(
                        ErrorArea::Selector,
                        "terminal event stream",
                    ));
                }
            }
        }
    }

    fn update_filter(&mut self) {
        self.filtered_indexes =
            filtered_indexes_by_query(&self.devices, &self.search_query, &self.matcher, |device| {
                device.identifier.as_str()
            });
        self.selected_filtered_index = 0;
    }

    fn navigate_up(&mut self) {
        if !self.filtered_indexes.is_empty() && self.selected_filtered_index > 0 {
            self.selected_filtered_index -= 1;
        }
    }

    fn navigate_down(&mut self) {
        if !self.filtered_indexes.is_empty()
            && self.selected_filtered_index < self.filtered_indexes.len() - 1
        {
            self.selected_filtered_index += 1;
        }
    }

    fn navigate_page(&mut self, dir: i32) {
        if self.filtered_indexes.is_empty() {
            return;
        }
        let len = self.filtered_indexes.len();
        if dir < 0 {
            self.selected_filtered_index = self
                .selected_filtered_index
                .saturating_sub(PAGE_SCROLL_SIZE);
        } else if dir > 0 {
            let target = self
                .selected_filtered_index
                .saturating_add(PAGE_SCROLL_SIZE);
            self.selected_filtered_index = target.min(len - 1);
        }
    }

    fn select_home(&mut self) {
        if !self.filtered_indexes.is_empty() {
            self.selected_filtered_index = 0;
        }
    }

    fn select_end(&mut self) {
        if !self.filtered_indexes.is_empty() {
            self.selected_filtered_index = self.filtered_indexes.len() - 1;
        }
    }

    fn add_char(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filter();
    }

    fn remove_char(&mut self) {
        self.search_query.pop();
        self.update_filter();
    }

    fn clear_search(&mut self) {
        self.search_query.clear();
        self.update_filter();
    }

    fn help_lines() -> Vec<String> {
        vec![
            "Move: Up/Down, Ctrl-P/Ctrl-N, PageUp/PageDown, Home/End".to_string(),
            "Select: Enter".to_string(),
            "Exit: Esc or Ctrl-C".to_string(),
            "Search: type to filter, Backspace, Ctrl-U clear".to_string(),
            "Refresh: Ctrl-R".to_string(),
            "Help: ? (press ? or Esc to close)".to_string(),
        ]
    }

    fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min, Percentage};

        let [_left_margin, content_area, _right_margin] = Layout::horizontal([
            Percentage(LAYOUT_MARGIN_PCT),
            Percentage(LAYOUT_CONTENT_WIDTH_PCT),
            Percentage(LAYOUT_MARGIN_PCT),
        ])
        .areas(area);

        // Keep existing top padding and search box, add a one-line footer below the list
        let [_top_padding, search_area, list_area] = Layout::vertical([
            Length(TOP_PADDING_HEIGHT),
            Length(SEARCH_BOX_HEIGHT),
            Min(MAIN_MIN_HEIGHT),
        ])
        .areas(content_area);

        self.render_search_box(search_area, buf);
        self.render_device_list(list_area, buf);
        self.render_help_popup(area, buf);
        self.render_error_popup(area, buf);
    }

    fn render_search_box(&self, area: Rect, buf: &mut Buffer) {
        let search_text = format!(" {}_", self.search_query);
        Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Search ")
                    .title_alignment(Alignment::Center)
                    .style(tailwind::BLUE.c300),
            )
            .fg(TEXT_COLOR)
            .render(area, buf);
    }

    fn render_device_list(&self, area: Rect, buf: &mut Buffer) {
        let items = self.filtered_indexes.iter().map(|&device_index| {
            let device = &self.devices[device_index];
            ListItem::new(device.identifier.clone())
        });

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Devices ")
                    .title_alignment(Alignment::Center)
                    .style(tailwind::BLUE.c300),
            )
            .style(TEXT_COLOR)
            .highlight_style(Style::default().bg(tailwind::GRAY.c600))
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(if self.filtered_indexes.is_empty() {
            None
        } else {
            Some(self.selected_filtered_index)
        });

        ratatui::widgets::StatefulWidget::render(list, area, buf, &mut list_state);
    }

    fn render_error_popup(&self, area: Rect, buf: &mut Buffer) {
        let Some(message) = &self.error_message else {
            return;
        };
        let popup = Popup {
            title: " Error ",
            lines: std::slice::from_ref(message),
            min_width: POPUP_MIN_WIDTH,
            min_height: POPUP_MIN_HEIGHT,
            max_width: Some(POPUP_MAX_WIDTH),
            max_height: None,
            text_style: Style::new().fg(tailwind::RED.c400).bold(),
            border_style: Style::new().fg(tailwind::RED.c400).bold(),
            text_alignment: Alignment::Center,
            title_alignment: Alignment::Center,
            wrap: Wrap { trim: true },
        };
        render_popup(area, buf, &popup);
    }

    fn render_help_popup(&self, area: Rect, buf: &mut Buffer) {
        if !self.help_visible {
            return;
        }
        let popup = Popup {
            title: " Help ",
            lines: &self.help_lines,
            min_width: HELP_POPUP_MIN_WIDTH,
            min_height: HELP_POPUP_MIN_HEIGHT,
            max_width: Some(HELP_POPUP_MAX_WIDTH),
            max_height: None,
            text_style: Style::new().fg(TEXT_COLOR),
            border_style: Style::new().fg(tailwind::BLUE.c300).bold(),
            text_alignment: Alignment::Left,
            title_alignment: Alignment::Center,
            wrap: Wrap { trim: false },
        };
        render_popup(area, buf, &popup);
    }
}

#[cfg(test)]
mod tests {
    use fuzzy_matcher::skim::SkimMatcherV2;

    use super::{DeviceSelector, DiscoveryIssue, DiscoveryResult, filtered_indexes_by_query};
    use std::{
        io,
        path::{Path, PathBuf},
    };

    #[test]
    fn discovery_issue_reports_no_devices_when_no_event_nodes_exist() {
        let result: DiscoveryResult<()> = DiscoveryResult::new();

        assert_eq!(result.issue(), Some(DiscoveryIssue::NoDevicesFound));
    }

    #[test]
    fn discovery_issue_reports_permission_guidance_when_all_devices_are_skipped() {
        let mut result: DiscoveryResult<()> = DiscoveryResult::new();
        result.event_nodes = 2;
        result.record_open_error(
            "/dev/input/event0",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );
        result.record_open_error(
            "/dev/input/event1",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );

        assert_eq!(
            result.issue(),
            Some(DiscoveryIssue::PermissionDenied { skipped: 2 })
        );
        assert_eq!(
            result.error_message(),
            Some(
                "found 2 input device node(s), but none were readable; check permissions for /dev/input/event*"
                    .to_string()
            )
        );
    }

    #[test]
    fn discovery_issue_reports_read_dir_failures() {
        let result: DiscoveryResult<()> = DiscoveryResult::read_dir_failed(
            "/dev/input",
            io::Error::new(io::ErrorKind::PermissionDenied, "read denied"),
        );

        assert_eq!(
            result.issue(),
            Some(DiscoveryIssue::ReadDir {
                path: PathBuf::from("/dev/input"),
                message: "read denied".to_string(),
            })
        );
    }

    #[test]
    fn discover_from_entries_counts_skips_and_filters_non_event_nodes() {
        let entries = vec![
            Ok(PathBuf::from("/dev/input/mice")),
            Ok(PathBuf::from("/dev/input/event1")),
            Err(io::Error::new(io::ErrorKind::Interrupted, "retry")),
            Ok(PathBuf::from("/dev/input/event0")),
        ];

        let result = DeviceSelector::discover_from_entries(entries, |path: &Path| {
            if path.ends_with("event1") {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "permission denied",
                ));
            }

            Ok(path.display().to_string())
        });

        assert_eq!(result.event_nodes, 2);
        assert_eq!(result.skipped, 1);
        assert!(result.had_read_dir_error);
        assert_eq!(result.devices, vec!["/dev/input/event0".to_string()]);
    }

    #[test]
    fn filtered_indexes_by_query_returns_all_items_for_empty_query() {
        let matcher = SkimMatcherV2::default();
        let identifiers = vec!["usb mouse", "gamepad"];

        let indexes = filtered_indexes_by_query(&identifiers, "", &matcher, |item| item);

        assert_eq!(indexes, vec![0, 1]);
    }

    #[test]
    fn filtered_indexes_by_query_returns_empty_when_nothing_matches() {
        let matcher = SkimMatcherV2::default();
        let identifiers = vec!["usb mouse", "gamepad"];

        let indexes = filtered_indexes_by_query(&identifiers, "keyboard", &matcher, |item| item);

        assert!(indexes.is_empty());
    }

    #[test]
    fn apply_discovery_resets_selection_and_updates_empty_state_error() {
        let mut selector = DeviceSelector {
            devices: Vec::new(),
            filtered_indexes: vec![0, 1],
            selected_filtered_index: 1,
            search_query: "mouse".to_string(),
            matcher: SkimMatcherV2::default(),
            error_message: None,
            help_visible: false,
            help_lines: Vec::new(),
        };

        let discovery = DiscoveryResult::read_dir_failed(
            "/dev/input",
            io::Error::new(io::ErrorKind::PermissionDenied, "read denied"),
        );

        selector.apply_discovery(discovery);

        assert!(selector.devices.is_empty());
        assert!(selector.filtered_indexes.is_empty());
        assert_eq!(selector.selected_filtered_index, 0);
        assert_eq!(
            selector.error_message,
            Some("unable to read /dev/input: read denied".to_string())
        );
        assert_eq!(selector.search_query, "mouse");
    }
}
