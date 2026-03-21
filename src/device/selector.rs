use std::{
    fs, io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use crossterm::event::{
    Event, EventStream as TermEventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
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
    message: String,
}

impl DiscoveryError {
    fn new(path: impl Into<PathBuf>, err: io::Error) -> Self {
        Self {
            path: path.into(),
            message: err.to_string(),
        }
    }
}

#[derive(Debug)]
struct DiscoveryStats {
    event_nodes: usize,
    permission_denied: usize,
    open_failed: usize,
    read_dir_failed: usize,
    sample_read_dir_error: Option<DiscoveryError>,
    sample_open_error: Option<DiscoveryError>,
}

impl DiscoveryStats {
    fn new() -> Self {
        Self {
            event_nodes: 0,
            permission_denied: 0,
            open_failed: 0,
            read_dir_failed: 0,
            sample_read_dir_error: None,
            sample_open_error: None,
        }
    }

    fn record_read_dir_error(&mut self, path: impl Into<PathBuf>, err: io::Error) {
        self.read_dir_failed += 1;
        if self.sample_read_dir_error.is_none() {
            self.sample_read_dir_error = Some(DiscoveryError::new(path, err));
        }
    }

    fn record_open_error(&mut self, path: impl Into<PathBuf>, err: io::Error) {
        let kind = err.kind();
        if kind == io::ErrorKind::PermissionDenied {
            self.permission_denied += 1;
            return;
        }

        self.open_failed += 1;
        if self.sample_open_error.is_none() {
            self.sample_open_error = Some(DiscoveryError::new(path, err));
        }
    }

    fn total_open_failures(&self) -> usize {
        self.permission_denied + self.open_failed
    }

    fn issue(&self, has_devices: bool) -> Option<DiscoveryIssue> {
        if has_devices {
            return None;
        }

        if self.event_nodes == 0 {
            return if let Some(error) = &self.sample_read_dir_error {
                Some(DiscoveryIssue::ReadDir {
                    path: error.path.clone(),
                    message: error.message.clone(),
                })
            } else {
                Some(DiscoveryIssue::NoDevicesFound)
            };
        }

        let skipped = self.total_open_failures();
        if skipped == 0 {
            return self
                .sample_read_dir_error
                .as_ref()
                .map(|error| DiscoveryIssue::ReadDir {
                    path: error.path.clone(),
                    message: error.message.clone(),
                });
        }

        if self.open_failed == 0 {
            return Some(DiscoveryIssue::PermissionDenied { skipped });
        }

        if let Some(error) = &self.sample_open_error {
            return Some(DiscoveryIssue::OpenFailed {
                skipped,
                path: error.path.clone(),
                message: error.message.clone(),
            });
        }

        Some(DiscoveryIssue::NoDevicesFound)
    }
}

#[derive(Debug)]
struct DiscoveryResult<T> {
    devices: Vec<T>,
    stats: DiscoveryStats,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectorMode {
    Browsing,
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectorCommand {
    Exit,
    Back,
    ToggleHelp,
    Refresh,
    Select,
    ClearSearch,
    DeleteChar,
    AddChar(char),
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Home,
    End,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectionAction {
    Refresh,
    Open(usize),
}

fn command_for(key: KeyEvent, mode: SelectorMode) -> SelectorCommand {
    match mode {
        SelectorMode::Help => match key.code {
            KeyCode::Esc | KeyCode::Char('?') => SelectorCommand::ToggleHelp,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::Exit
            }
            _ => SelectorCommand::None,
        },
        SelectorMode::Browsing => match key.code {
            KeyCode::Enter => SelectorCommand::Select,
            KeyCode::Esc => SelectorCommand::Back,
            KeyCode::Char('?') => SelectorCommand::ToggleHelp,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::Exit
            }
            KeyCode::Up => SelectorCommand::MoveUp,
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::MoveUp
            }
            KeyCode::Down => SelectorCommand::MoveDown,
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::MoveDown
            }
            KeyCode::PageUp => SelectorCommand::PageUp,
            KeyCode::PageDown => SelectorCommand::PageDown,
            KeyCode::Home => SelectorCommand::Home,
            KeyCode::End => SelectorCommand::End,
            KeyCode::Backspace => SelectorCommand::DeleteChar,
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::ClearSearch
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::Refresh
            }
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::SHIFT || key.modifiers.is_empty() =>
            {
                SelectorCommand::AddChar(c)
            }
            _ => SelectorCommand::None,
        },
    }
}

impl<T> DiscoveryResult<T> {
    fn new() -> Self {
        Self {
            devices: Vec::new(),
            stats: DiscoveryStats::new(),
        }
    }

    fn read_dir_failed(path: impl Into<PathBuf>, err: io::Error) -> Self {
        let mut result = Self::new();
        result.stats.record_read_dir_error(path, err);
        result
    }

    fn issue(&self) -> Option<DiscoveryIssue> {
        self.stats.issue(!self.devices.is_empty())
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
    mode: SelectorMode,
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
            mode: SelectorMode::Browsing,
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

                    result.stats.event_nodes += 1;
                    match open_device(&path) {
                        Ok(device) => result.devices.push(device),
                        Err(err) => result.stats.record_open_error(path, err),
                    }
                }
                Err(err) => result.stats.record_read_dir_error(INPUT_DIR, err),
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
                    if let Some(state) = selector.handle_key_press(key) {
                        return Ok(state);
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

    fn handle_key_press(&mut self, key: KeyEvent) -> Option<State> {
        let mode = self.mode;
        if mode == SelectorMode::Browsing {
            self.error_message = None;
        }
        self.apply_command(command_for(key, mode))
    }

    fn apply_command(&mut self, command: SelectorCommand) -> Option<State> {
        match command {
            SelectorCommand::Exit => Some(State::Exit),
            SelectorCommand::Back => self.back(),
            SelectorCommand::ToggleHelp => {
                self.toggle_help();
                None
            }
            SelectorCommand::Refresh => {
                self.refresh_devices();
                None
            }
            SelectorCommand::Select => self.select_or_refresh(),
            SelectorCommand::ClearSearch => {
                self.clear_search();
                None
            }
            SelectorCommand::DeleteChar => {
                self.remove_char();
                None
            }
            SelectorCommand::AddChar(c) => {
                self.add_char(c);
                None
            }
            SelectorCommand::MoveUp => {
                self.move_selection_by(-1);
                None
            }
            SelectorCommand::MoveDown => {
                self.move_selection_by(1);
                None
            }
            SelectorCommand::PageUp => {
                self.move_selection_by(-(PAGE_SCROLL_SIZE as i32));
                None
            }
            SelectorCommand::PageDown => {
                self.move_selection_by(PAGE_SCROLL_SIZE as i32);
                None
            }
            SelectorCommand::Home => {
                self.select_index(0);
                None
            }
            SelectorCommand::End => {
                if let Some(last_index) = self.filtered_indexes.len().checked_sub(1) {
                    self.select_index(last_index);
                }
                None
            }
            SelectorCommand::None => None,
        }
    }

    fn back(&mut self) -> Option<State> {
        if self.search_query.is_empty() {
            Some(State::Exit)
        } else {
            self.clear_search();
            None
        }
    }

    fn move_selection_by(&mut self, delta: i32) {
        let len = self.filtered_indexes.len();
        if len == 0 || delta == 0 {
            return;
        }

        let max_index = len - 1;
        let target = self.selected_filtered_index as i32 + delta;
        self.selected_filtered_index = target.clamp(0, max_index as i32) as usize;
    }

    fn select_index(&mut self, index: usize) {
        if let Some(max_index) = self.filtered_indexes.len().checked_sub(1) {
            self.selected_filtered_index = index.min(max_index);
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

    fn selection_action(&self) -> Option<SelectionAction> {
        if self.filtered_indexes.is_empty() {
            return Some(SelectionAction::Refresh);
        }

        self.filtered_indexes
            .get(self.selected_filtered_index)
            .copied()
            .map(SelectionAction::Open)
    }

    fn select_or_refresh(&mut self) -> Option<State> {
        match self.selection_action() {
            Some(SelectionAction::Refresh) => {
                self.refresh_devices();
                None
            }
            Some(SelectionAction::Open(index)) => {
                Some(State::Monitor(Box::new(self.devices.swap_remove(index))))
            }
            None => None,
        }
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
        self.mode = match self.mode {
            SelectorMode::Browsing => SelectorMode::Help,
            SelectorMode::Help => SelectorMode::Browsing,
        };
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
        if self.mode != SelectorMode::Help {
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
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use fuzzy_matcher::skim::SkimMatcherV2;

    use super::{
        DeviceSelector, DiscoveryIssue, DiscoveryResult, SelectionAction, SelectorCommand,
        SelectorMode, State, command_for, filtered_indexes_by_query,
    };
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
        result.stats.event_nodes = 2;
        result.stats.record_open_error(
            "/dev/input/event0",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );
        result.stats.record_open_error(
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
    fn discovery_issue_reports_open_failures_when_causes_are_mixed() {
        let mut result: DiscoveryResult<()> = DiscoveryResult::new();
        result.stats.event_nodes = 2;
        result.stats.record_open_error(
            "/dev/input/event0",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );
        result.stats.record_open_error(
            "/dev/input/event1",
            io::Error::new(io::ErrorKind::NotFound, "device disappeared"),
        );

        assert_eq!(
            result.issue(),
            Some(DiscoveryIssue::OpenFailed {
                skipped: 2,
                path: PathBuf::from("/dev/input/event1"),
                message: "device disappeared".to_string(),
            })
        );
    }

    #[test]
    fn discovery_issue_prefers_open_failures_over_partial_read_dir_errors() {
        let mut result: DiscoveryResult<()> = DiscoveryResult::new();
        result.stats.event_nodes = 1;
        result.stats.record_read_dir_error(
            "/dev/input",
            io::Error::new(io::ErrorKind::Interrupted, "retry"),
        );
        result.stats.record_open_error(
            "/dev/input/event0",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );

        assert_eq!(
            result.issue(),
            Some(DiscoveryIssue::PermissionDenied { skipped: 1 })
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

        assert_eq!(result.stats.event_nodes, 2);
        assert_eq!(result.stats.total_open_failures(), 1);
        assert_eq!(result.stats.read_dir_failed, 1);
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
            mode: SelectorMode::Browsing,
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

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn shifted_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
    }

    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn selector_with(filtered_indexes: Vec<usize>, search_query: &str) -> DeviceSelector {
        DeviceSelector {
            devices: Vec::new(),
            filtered_indexes,
            selected_filtered_index: 0,
            search_query: search_query.to_string(),
            matcher: SkimMatcherV2::default(),
            error_message: None,
            mode: SelectorMode::Browsing,
            help_lines: Vec::new(),
        }
    }

    #[test]
    fn command_for_ctrl_c_exits_from_any_mode() {
        for mode in [SelectorMode::Browsing, SelectorMode::Help] {
            assert_eq!(command_for(ctrl_char('c'), mode), SelectorCommand::Exit);
        }
    }

    #[test]
    fn command_for_escape_depends_on_mode() {
        assert_eq!(
            command_for(key(KeyCode::Esc), SelectorMode::Browsing),
            SelectorCommand::Back
        );
        assert_eq!(
            command_for(key(KeyCode::Esc), SelectorMode::Help),
            SelectorCommand::ToggleHelp
        );
    }

    #[test]
    fn command_for_maps_navigation_keys_to_explicit_variants() {
        assert_eq!(
            command_for(key(KeyCode::Up), SelectorMode::Browsing),
            SelectorCommand::MoveUp
        );
        assert_eq!(
            command_for(ctrl_char('p'), SelectorMode::Browsing),
            SelectorCommand::MoveUp
        );
        assert_eq!(
            command_for(key(KeyCode::Down), SelectorMode::Browsing),
            SelectorCommand::MoveDown
        );
        assert_eq!(
            command_for(ctrl_char('n'), SelectorMode::Browsing),
            SelectorCommand::MoveDown
        );
        assert_eq!(
            command_for(key(KeyCode::PageUp), SelectorMode::Browsing),
            SelectorCommand::PageUp
        );
        assert_eq!(
            command_for(key(KeyCode::PageDown), SelectorMode::Browsing),
            SelectorCommand::PageDown
        );
    }

    #[test]
    fn command_for_only_adds_plain_and_shifted_characters() {
        assert_eq!(
            command_for(key(KeyCode::Char('a')), SelectorMode::Browsing),
            SelectorCommand::AddChar('a')
        );
        assert_eq!(
            command_for(shifted_char('A'), SelectorMode::Browsing),
            SelectorCommand::AddChar('A')
        );
        assert_eq!(
            command_for(
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT),
                SelectorMode::Browsing
            ),
            SelectorCommand::None
        );
    }

    #[test]
    fn back_exits_only_when_search_is_empty() {
        let mut selector = selector_with(vec![0], "mouse");

        assert!(selector.back().is_none());
        assert!(selector.search_query.is_empty());

        assert!(matches!(selector.back(), Some(State::Exit)));
    }

    #[test]
    fn selection_action_refreshes_when_no_results_exist() {
        let selector = selector_with(Vec::new(), "");

        assert_eq!(selector.selection_action(), Some(SelectionAction::Refresh));
    }

    #[test]
    fn selection_action_uses_selected_filtered_index() {
        let mut selector = selector_with(vec![2, 5, 7], "");
        selector.selected_filtered_index = 1;

        assert_eq!(selector.selection_action(), Some(SelectionAction::Open(5)));
    }
}
