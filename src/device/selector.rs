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

const LABEL_COLOR: ratatui::style::Color = tailwind::SLATE.c200;

#[derive(Debug)]
struct DeviceInfo {
    _device: Device,
    name: String,
    path: String,
}

pub struct DeviceSelector {
    devices: Vec<DeviceInfo>,
    filtered_devices: Vec<usize>, // indices into devices
    selected_index: usize,
    search_query: String,
    matcher: SkimMatcherV2,
    list_state: ListState,
}

impl DeviceSelector {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let devices: Vec<DeviceInfo> = evdev::enumerate()
            .map(|(path, device)| {
                let name = device.name().unwrap_or("Unknown Device").to_string();
                let path_str = path.to_string_lossy().to_string();
                DeviceInfo {
                    _device: device,
                    name,
                    path: path_str,
                }
            })
            .collect();

        if devices.is_empty() {
            return Err("No input devices found!".into());
        }

        let filtered_devices = (0..devices.len()).collect();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Ok(Self {
            devices,
            filtered_devices,
            selected_index: 0,
            search_query: String::new(),
            matcher: SkimMatcherV2::default(),
            list_state,
        })
    }

    pub async fn select_device(
        terminal: &mut DefaultTerminal,
    ) -> Result<Device, Box<dyn std::error::Error>> {
        let mut selector = Self::new()?;
        let mut term_events = TermEventStream::new();

        loop {
            // Draw the UI
            terminal.draw(|frame| {
                selector.render(frame.area(), frame.buffer_mut());
            })?;

            // Handle terminal events
            if let Some(Ok(event)) = term_events.next().await {
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => {
                                return Err("User cancelled device selection".into());
                            }
                            KeyCode::Enter => {
                                if !selector.filtered_devices.is_empty() {
                                    let device_index =
                                        selector.filtered_devices[selector.selected_index];
                                    let device_info = &selector.devices[device_index];

                                    // Open and return the device
                                    let device = Device::open(&device_info.path)?;
                                    return Ok(device);
                                }
                            }
                            KeyCode::Up => {
                                selector.navigate_up();
                            }
                            KeyCode::Down => {
                                selector.navigate_down();
                            }
                            KeyCode::Backspace => {
                                selector.remove_char();
                            }
                            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                selector.navigate_up();
                            }
                            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                selector.navigate_down();
                            }
                            KeyCode::Char(c) => {
                                selector.add_char(c);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_devices = (0..self.devices.len()).collect();
        } else {
            let mut scored_devices: Vec<(usize, i64)> = self
                .devices
                .iter()
                .enumerate()
                .filter_map(|(i, device)| {
                    self.matcher
                        .fuzzy_match(&device.name, &self.search_query)
                        .map(|score| (i, score))
                })
                .collect();

            // Sort by score (higher is better)
            scored_devices.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered_devices = scored_devices.into_iter().map(|(i, _)| i).collect();
        }

        // Reset selection to first item
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    fn navigate_up(&mut self) {
        if !self.filtered_devices.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    fn navigate_down(&mut self) {
        if !self.filtered_devices.is_empty()
            && self.selected_index < self.filtered_devices.len() - 1
        {
            self.selected_index += 1;
            self.list_state.select(Some(self.selected_index));
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

        // Add horizontal padding - center the content with margins on left/right
        let horizontal_layout = Layout::horizontal([
            Percentage(20), // left margin
            Percentage(60), // content area
            Percentage(20), // right margin
        ]);
        let [_left_margin, content_area, _right_margin] = horizontal_layout.areas(area);

        let layout = Layout::vertical([
            Length(2), // top padding
            Length(3), // search input
            Min(5),    // device list
            Length(1), // bottom padding
            Length(2), // footer
        ]);
        let [
            _top_padding,
            search_area,
            list_area,
            _bottom_padding,
            footer_area,
        ] = layout.areas(content_area);

        // Search input
        let search_text = format!(" {}_", self.search_query);
        Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Search ")
                    .title_alignment(Alignment::Center)
                    .style(tailwind::BLUE.c300),
            )
            .fg(LABEL_COLOR)
            .render(search_area, buf);

        // Device list
        let items: Vec<ListItem> = self
            .filtered_devices
            .iter()
            .map(|&device_index| {
                let device = &self.devices[device_index];
                let content = format!("{} ({})", device.name, device.path);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Devices ")
                    .title_alignment(Alignment::Center)
                    .style(tailwind::BLUE.c300),
            )
            .style(LABEL_COLOR)
            .highlight_style(Style::default().bg(tailwind::GRAY.c600))
            .highlight_symbol("> ");

        ratatui::widgets::StatefulWidget::render(list, list_area, buf, &mut self.list_state);

        // Footer
        Paragraph::new(
            "Use ↑↓/Ctrl+P/Ctrl+N to navigate, type to search, Enter to select, ESC to quit",
        )
        .alignment(Alignment::Center)
        .fg(LABEL_COLOR)
        .render(footer_area, buf);
    }
}
