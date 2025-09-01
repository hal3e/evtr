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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Widget},
};

const TEXT_COLOR: ratatui::style::Color = tailwind::SLATE.c200;

#[derive(Debug)]
struct DeviceInfo {
    device: Device,
    identifier: String,
}

pub struct DeviceSelector {
    devices: Vec<DeviceInfo>,
    filtered_indexes: Vec<usize>,
    selected_filtered_index: usize,
    search_query: String,
    matcher: SkimMatcherV2,
}

impl DeviceSelector {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let devices: Vec<DeviceInfo> = evdev::enumerate()
            .map(|(path, device)| {
                let name = device.name().unwrap_or("Unknown Device").to_string();
                let path = path.to_string_lossy().to_string();

                DeviceInfo {
                    device,
                    identifier: format!("{name} ({path})"),
                }
            })
            .collect();

        if devices.is_empty() {
            return Err("no input devices found!".into());
        }

        let filtered_indexes = (0..devices.len()).collect();

        Ok(Self {
            devices,
            filtered_indexes,
            selected_filtered_index: 0,
            search_query: String::new(),
            matcher: SkimMatcherV2::default(),
        })
    }

    pub async fn run(
        terminal: &mut DefaultTerminal,
    ) -> Result<Option<Device>, Box<dyn std::error::Error>> {
        let mut selector = Self::new()?;
        let mut term_events = TermEventStream::new();

        loop {
            terminal.draw(|frame| {
                selector.render(frame.area(), frame.buffer_mut());
            })?;

            if let Some(Ok(Event::Key(key))) = term_events.next().await
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Enter if !selector.filtered_indexes.is_empty() => {
                        return Ok(selector
                            .devices
                            .into_iter()
                            .nth(selector.filtered_indexes[selector.selected_filtered_index])
                            .map(|DeviceInfo { device, .. }| device));
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(None);
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
                    KeyCode::Backspace => {
                        selector.remove_char();
                    }
                    KeyCode::Char(c)
                        if key.modifiers == KeyModifiers::SHIFT || key.modifiers.is_empty() =>
                    {
                        selector.add_char(c);
                    }
                    _ => {}
                }
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

    fn add_char(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filter();
    }

    fn remove_char(&mut self) {
        self.search_query.pop();
        self.update_filter();
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min, Percentage};

        const LAYOUT_MARGIN: u16 = 20;
        const LAYOUT_CONTENT_WIDTH: u16 = 60;

        let horizontal_layout = Layout::horizontal([
            Percentage(LAYOUT_MARGIN),
            Percentage(LAYOUT_CONTENT_WIDTH),
            Percentage(LAYOUT_MARGIN),
        ]);
        let [_left_margin, content_area, _right_margin] = horizontal_layout.areas(area);

        let layout = Layout::vertical([Length(1), Length(3), Min(3)]);
        let [_top_padding, search_area, list_area] = layout.areas(content_area);

        self.render_search_box(search_area, buf);
        self.render_device_list(list_area, buf);
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
}
