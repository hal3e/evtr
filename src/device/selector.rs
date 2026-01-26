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

use crate::{
    device::popup::{Popup, render_popup},
    error::{Error, Result},
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
const POPUP_MAX_HEIGHT: u16 = 5;
const HELP_POPUP_MIN_WIDTH: u16 = 30;
const HELP_POPUP_MIN_HEIGHT: u16 = 6;
const HELP_POPUP_MAX_WIDTH: u16 = 80;

#[derive(Debug)]
pub struct DeviceInfo {
    pub device: Device,
    pub identifier: String,
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
    fn new(error_message: Option<String>) -> Result<Self> {
        let devices = Self::load_devices()?;

        let filtered_indexes = (0..devices.len()).collect();

        Ok(Self {
            devices,
            filtered_indexes,
            selected_filtered_index: 0,
            search_query: String::new(),
            matcher: SkimMatcherV2::default(),
            error_message,
            help_visible: false,
            help_lines: Self::help_lines(),
        })
    }

    fn load_devices() -> Result<Vec<DeviceInfo>> {
        let mut devices: Vec<DeviceInfo> = evdev::enumerate()
            .map(|(path, device)| {
                let name = device.name().unwrap_or("Unknown Device").to_string();
                let path = path.to_string_lossy().to_string();

                DeviceInfo {
                    device,
                    identifier: format!("{name} ({path})"),
                }
            })
            .collect();

        devices.sort_unstable_by(|a, b| {
            a.identifier
                .to_lowercase()
                .cmp(&b.identifier.to_lowercase())
        });

        if devices.is_empty() {
            return Err(Error::NoDevicesFound);
        }

        Ok(devices)
    }

    fn refresh_devices(&mut self) -> Result<()> {
        self.devices = Self::load_devices()?;
        self.update_filter();
        Ok(())
    }

    pub async fn run(
        terminal: &mut DefaultTerminal,
        error_message: Option<String>,
    ) -> Result<Option<DeviceInfo>> {
        let mut selector = Self::new(error_message)?;
        let mut term_events = TermEventStream::new();

        loop {
            terminal
                .draw(|frame| {
                    selector.render(frame.area(), frame.buffer_mut());
                })
                .map_err(|err| Error::io("selector draw", err))?;

            match term_events.next().await {
                Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                    if selector.help_visible {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') => {
                                selector.help_visible = false;
                            }
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                return Ok(None);
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
                        KeyCode::Enter if !selector.filtered_indexes.is_empty() => {
                            if let Some(&index) = selector
                                .filtered_indexes
                                .get(selector.selected_filtered_index)
                            {
                                return Ok(Some(selector.devices.swap_remove(index)));
                            }
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(None);
                        }
                        KeyCode::Esc => {
                            if selector.search_query.is_empty() {
                                return Ok(None);
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
                            if let Err(err) = selector.refresh_devices() {
                                selector.error_message = Some(err.to_string());
                            }
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
                Some(Err(err)) => return Err(Error::terminal("terminal event stream", err)),
                None => return Err(Error::stream_ended("terminal event stream")),
            }
        }
    }

    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indexes = (0..self.devices.len()).collect();
        } else {
            let mut scored_devices: Vec<(usize, i64)> = self
                .devices
                .iter()
                .enumerate()
                .filter_map(|(i, device)| {
                    self.matcher
                        .fuzzy_match(&device.identifier, &self.search_query)
                        .map(|score| (i, score))
                })
                .collect();

            // Sort by score (higher is better)
            scored_devices.sort_unstable_by(|a, b| b.1.cmp(&a.1));
            self.filtered_indexes = scored_devices.into_iter().map(|(i, _)| i).collect();
        }
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
            max_height: Some(POPUP_MAX_HEIGHT),
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
