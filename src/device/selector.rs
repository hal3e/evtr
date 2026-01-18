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
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Widget, Wrap},
};

use crate::error::{Error, Result};

const TEXT_COLOR: ratatui::style::Color = tailwind::SLATE.c200;

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
}

impl DeviceSelector {
    fn new(error_message: Option<String>) -> Result<Self> {
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

        let filtered_indexes = (0..devices.len()).collect();

        Ok(Self {
            devices,
            filtered_indexes,
            selected_filtered_index: 0,
            search_query: String::new(),
            matcher: SkimMatcherV2::default(),
            error_message,
        })
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
                    if selector.error_message.is_some() {
                        selector.error_message = None;
                    }
                    match key.code {
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
        const PAGE: usize = 10;
        let len = self.filtered_indexes.len();
        if dir < 0 {
            self.selected_filtered_index = self.selected_filtered_index.saturating_sub(PAGE);
        } else if dir > 0 {
            let target = self.selected_filtered_index.saturating_add(PAGE);
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

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min, Percentage};

        const LAYOUT_MARGIN: u16 = 20;
        const LAYOUT_CONTENT_WIDTH: u16 = 60;

        let [_left_margin, content_area, _right_margin] = Layout::horizontal([
            Percentage(LAYOUT_MARGIN),
            Percentage(LAYOUT_CONTENT_WIDTH),
            Percentage(LAYOUT_MARGIN),
        ])
        .areas(area);

        // Keep existing top padding and search box, add a one-line footer below the list
        let [_top_padding, search_area, main_area] =
            Layout::vertical([Length(1), Length(3), Min(3)]).areas(content_area);

        let [list_area, footer_area] = Layout::vertical([Min(1), Length(1)]).areas(main_area);

        self.render_search_box(search_area, buf);
        self.render_device_list(list_area, buf);
        self.render_footer(footer_area, buf);
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

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let total = self.devices.len();
        let filtered = self.filtered_indexes.len();
        let footer_text = format!(
            "Enter: select | Esc: clear/exit | Ctrl-C: quit | ↑/↓/PgUp/PgDn/Home/End: navigate | Ctrl-U: clear | Matches: {}/{}",
            filtered, total
        );
        Paragraph::new(footer_text)
            .style(Style::new().fg(tailwind::SLATE.c200).bold())
            .alignment(Alignment::Center)
            .render(area, buf);
    }

    fn render_error_popup(&self, area: Rect, buf: &mut Buffer) {
        let Some(message) = &self.error_message else {
            return;
        };

        if area.width < 10 || area.height < 3 {
            return;
        }

        let width = area.width.min(80);
        let height = area.height.min(5).max(3);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup_area = Rect::new(x, y, width, height);

        Clear.render(popup_area, buf);

        Paragraph::new(message.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Error ")
                    .style(Style::new().fg(tailwind::RED.c400).bold()),
            )
            .style(Style::new().fg(tailwind::RED.c400).bold())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(popup_area, buf);
    }
}
