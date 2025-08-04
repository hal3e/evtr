mod devices;

use std::{collections::HashMap, time::Duration};

use evdev::{AbsoluteAxisCode, Device, EventType};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize, palette::tailwind},
    text::Line,
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Widget},
};

const GAUGE_COLOR: Color = tailwind::BLUE.c400;
const LABEL_COLOR: Color = tailwind::SLATE.c200;

#[derive(Debug)]
struct AxisInfo {
    code: u16,
    name: String,
    min: i32,
    max: i32,
}

#[derive(Debug)]
struct ButtonInfo {
    code: u16,
    name: String,
}

#[derive(Debug)]
struct DeviceInfo {
    _device: Device, //TODO: use device and do not open again
    name: String,
    path: String,
}

struct DeviceSelector {
    devices: Vec<DeviceInfo>,
    filtered_devices: Vec<usize>, // indices into devices
    selected_index: usize,
    search_query: String,
    matcher: SkimMatcherV2,
    list_state: ListState,
}

impl std::fmt::Debug for DeviceSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceSelector")
            .field("devices_count", &self.devices.len())
            .field("filtered_count", &self.filtered_devices.len())
            .field("selected_index", &self.selected_index)
            .field("search_query", &self.search_query)
            .finish()
    }
}

#[derive(Debug)]
struct App {
    state: AppState,
    device: Option<Device>,
    axis_values: HashMap<u16, i32>,
    axes: Vec<AxisInfo>,
    button_states: HashMap<u16, bool>,
    buttons: Vec<ButtonInfo>,
    device_selector: Option<DeviceSelector>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum AppState {
    #[default]
    DeviceSelection,
    Running,
    BackToSelection,
    Quitting,
}

fn main() -> Result<(), &'static str> {
    let terminal = ratatui::init();
    let result = run_app_loop(terminal);
    ratatui::restore();

    result
}

fn run_app_loop(mut terminal: DefaultTerminal) -> Result<(), &'static str> {
    loop {
        // Create app with device selection state
        let device_selector = DeviceSelector::new().ok_or("No joystick devices found!")?;

        let app = App {
            state: AppState::DeviceSelection,
            device: None,
            axis_values: HashMap::new(),
            axes: Vec::new(),
            button_states: HashMap::new(),
            buttons: Vec::new(),
            device_selector: Some(device_selector),
        };

        let should_continue = app.run(&mut terminal)?;
        if !should_continue {
            break;
        }
        // Continue loop to show device selection again
    }

    Ok(())
}

fn create_app_with_device(device: evdev::Device) -> App {
    // Get all available axes from the device
    let mut axes = Vec::new();
    let mut initial_axis_values = HashMap::new();
    if let Some(abs_axes) = device.supported_absolute_axes() {
        for axis_type in abs_axes.iter() {
            let code = axis_type.0;
            let name = format!("{:?}", AbsoluteAxisCode(code));

            // Try to get axis info including min/max values and current value
            let (min, max, current_value) = if let Ok(abs_state) = device.get_abs_state() {
                if code < abs_state.len() as u16 {
                    let info = &abs_state[code as usize];
                    let min = info.minimum;
                    let max = info.maximum;
                    let current = info.value;
                    (min, max, current)
                } else {
                    // Default range if index out of bounds
                    (-32768, 32767, 0)
                }
            } else {
                // Default range if no state available
                (-32768, 32767, 0)
            };

            axes.push(AxisInfo {
                code,
                name,
                min,
                max,
            });
            initial_axis_values.insert(code, current_value);
        }
    }

    // Get all available buttons from the device
    let mut buttons = Vec::new();
    let mut initial_button_states = HashMap::new();
    if let Some(keys) = device.supported_keys() {
        for key in keys.iter() {
            let code = key.0;

            buttons.push(ButtonInfo {
                code,
                name: format!("{key:?}"),
            });
            initial_button_states.insert(code, false);
        }
    }

    App {
        state: AppState::Running,
        device: Some(device),
        axis_values: initial_axis_values,
        axes,
        button_states: initial_button_states,
        buttons,
        device_selector: None,
    }
}

impl DeviceSelector {
    fn new() -> Option<Self> {
        let devices: Vec<DeviceInfo> = evdev::enumerate()
            .filter(|(_, device)| devices::is_joystick_device(device))
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
            return None;
        }

        let filtered_devices = (0..devices.len()).collect();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Some(Self {
            devices,
            filtered_devices,
            selected_index: 0,
            search_query: String::new(),
            matcher: SkimMatcherV2::default(),
            list_state,
        })
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
}

impl App {
    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<bool, &'static str> {
        while self.state != AppState::Quitting && self.state != AppState::BackToSelection {
            self.handle_events()?;
            terminal
                .draw(|frame| frame.render_widget(&self, frame.area()))
                .map_err(|_| "Failed to draw terminal")?;
            self.update();
        }

        // Return true if we should continue (back to selection), false if we should quit
        Ok(self.state == AppState::BackToSelection)
    }

    fn update(&mut self) {
        if self.state == AppState::Quitting {
            return;
        }

        if let Some(ref mut device) = self.device {
            if let Ok(events) = device.fetch_events() {
                for event in events {
                    match event.event_type() {
                        EventType::ABSOLUTE => {
                            let code = event.code();
                            let value = event.value();
                            self.axis_values.insert(code, value);
                        }
                        EventType::KEY => {
                            let code = event.code();
                            let pressed = event.value() != 0;
                            self.button_states.insert(code, pressed);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn handle_events(&mut self) -> Result<(), &'static str> {
        let timeout = Duration::from_millis(50);

        if event::poll(timeout).map_err(|_| "Failed to poll events")? {
            if let Event::Key(key) = event::read().map_err(|_| "Failed to read key event")? {
                if key.kind == KeyEventKind::Press {
                    match self.state {
                        AppState::DeviceSelection => {
                            self.handle_device_selection_input(key.code, key.modifiers)?;
                        }
                        AppState::Running => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_device_selection_input(
        &mut self,
        key_code: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<(), &'static str> {
        if let Some(selector) = &mut self.device_selector {
            match key_code {
                KeyCode::Esc => {
                    self.state = AppState::Quitting;
                }
                KeyCode::Enter => {
                    if !selector.filtered_devices.is_empty() {
                        let device_index = selector.filtered_devices[selector.selected_index];
                        let device_info = &selector.devices[device_index];

                        // Reopen the device to get an owned copy
                        let device = Device::open(&device_info.path)
                            .map_err(|_| "Failed to reopen device")?;

                        device
                            .set_nonblocking(true)
                            .map_err(|_| "Failed to set non-blocking mode")?;

                        *self = create_app_with_device(device);
                        self.state = AppState::Running;
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
                KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => {
                    selector.navigate_up();
                }
                KeyCode::Char('n') if modifiers.contains(KeyModifiers::CONTROL) => {
                    selector.navigate_down();
                }
                KeyCode::Char(c) => {
                    selector.add_char(c);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn quit(&mut self) {
        match self.state {
            AppState::DeviceSelection => {
                self.state = AppState::Quitting;
            }
            AppState::Running => {
                self.state = AppState::BackToSelection;
            }
            _ => {}
        }
    }

    fn get_axis_value(&self, axis_code: u16) -> (i32, f64) {
        let value = self.axis_values.get(&axis_code).copied().unwrap_or(0);

        // Find the axis info to get min/max values
        if let Some(axis_info) = self.axes.iter().find(|a| a.code == axis_code) {
            let range = (axis_info.max - axis_info.min) as f64;
            let normalized = if range > 0.0 {
                ((value - axis_info.min) as f64) / range
            } else {
                0.5 // Default to center if no range
            };
            (value, normalized.clamp(0.0, 1.0))
        } else {
            // Fallback to old normalization if axis info not found
            let normalized = ((value + 32768) as f64) / 65535.0;
            (value, normalized.clamp(0.0, 1.0))
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.state {
            AppState::DeviceSelection => {
                if let Some(selector) = &self.device_selector {
                    self.render_device_selection(selector, area, buf);
                }
            }
            AppState::Running => {
                self.render_joystick_monitor(area, buf);
            }
            _ => {}
        }
    }
}

impl App {
    fn render_device_selection(&self, selector: &DeviceSelector, area: Rect, buf: &mut Buffer) {
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
        let search_text = format!(" {}_", selector.search_query);
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
        let items: Vec<ListItem> = selector
            .filtered_devices
            .iter()
            .map(|&device_index| {
                let device = &selector.devices[device_index];
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

        ratatui::widgets::StatefulWidget::render(
            list,
            list_area,
            buf,
            &mut selector.list_state.clone(),
        );

        // Footer
        Paragraph::new(
            "Use ↑↓/Ctrl+P/Ctrl+N to navigate, type to search, Enter to select, ESC to quit",
        )
        .alignment(Alignment::Center)
        .fg(LABEL_COLOR)
        .render(footer_area, buf);
    }

    fn render_joystick_monitor(&self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Ratio};

        let button_rows = if self.buttons.is_empty() {
            0
        } else {
            // Calculate rows needed for buttons (assuming 6 buttons per row)
            self.buttons.len().div_ceil(6)
        };

        let layout = Layout::vertical([
            Length(2),                          // header
            Length(self.axes.len() as u16 * 4), // gauges
            Length(button_rows as u16 * 3 + 2), // buttons + spacing
            Length(1),                          // footer
        ]);
        let [header_area, gauge_area, button_area, footer_area] = layout.areas(area);

        render_header(header_area, buf);
        render_footer(footer_area, buf);

        // Render gauges for axes
        if !self.axes.is_empty() {
            let num_axes = self.axes.len();
            let constraints: Vec<Constraint> =
                (0..num_axes).map(|_| Ratio(1, num_axes as u32)).collect();
            let layout = Layout::vertical(constraints);
            let gauge_areas = layout.split(gauge_area);

            for (i, axis_info) in self.axes.iter().enumerate() {
                if i < gauge_areas.len() {
                    self.render_gauge(axis_info.code, &axis_info.name, gauge_areas[i], buf);
                }
            }
        }

        // Render buttons
        if !self.buttons.is_empty() {
            self.render_buttons(button_area, buf);
        }
    }
}

fn render_header(area: Rect, buf: &mut Buffer) {
    Paragraph::new("Joystick Monitor - Live Axis Values")
        .bold()
        .alignment(Alignment::Center)
        .fg(LABEL_COLOR)
        .render(area, buf);
}

fn render_footer(area: Rect, buf: &mut Buffer) {
    Paragraph::new("Press 'q' or ESC to quit")
        .alignment(Alignment::Center)
        .fg(LABEL_COLOR)
        .bold()
        .render(area, buf);
}

impl App {
    fn render_gauge(&self, axis_code: u16, title: &str, area: Rect, buf: &mut Buffer) {
        let (value, normalized) = self.get_axis_value(axis_code);

        Gauge::default()
            .block(title_block(title))
            .gauge_style(GAUGE_COLOR)
            .ratio(normalized)
            .label(format!("{value}"))
            .render(area, buf);
    }

    fn render_buttons(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 {
            return;
        }

        // Calculate button layout (6 buttons per row)
        let buttons_per_row = 6usize;
        let button_width = area.width / buttons_per_row as u16;
        let rows = self.buttons.len().div_ceil(buttons_per_row);

        for (i, button_info) in self.buttons.iter().enumerate() {
            let row = i / buttons_per_row;
            let col = i % buttons_per_row;

            if row < rows && (row * 3 + 1) < area.height as usize {
                let x = area.x + (col as u16 * button_width);
                let y = area.y + (row as u16 * 3) + 1;
                let width = button_width.saturating_sub(1);
                let height = 3;

                if x + width <= area.x + area.width && y + height <= area.y + area.height {
                    let button_area = Rect::new(x, y, width, height);
                    self.render_button(button_info, button_area, buf);
                }
            }
        }
    }

    fn render_button(&self, button_info: &ButtonInfo, area: Rect, buf: &mut Buffer) {
        let is_pressed = self
            .button_states
            .get(&button_info.code)
            .copied()
            .unwrap_or(false);

        let mut block = Block::default().borders(Borders::ALL);

        if is_pressed {
            block = block.bg(tailwind::RED.c400);
        }

        // Clean up button name (remove "KEY_" prefix if present)
        let clean_name = &button_info.name;

        let text = if clean_name.len() > area.width as usize {
            &clean_name[..area.width as usize]
        } else {
            clean_name
        };

        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .fg(LABEL_COLOR)
            .render(area, buf);
    }
}

fn title_block(title: &str) -> Block {
    let title = Line::from(title).centered();

    Block::new()
        .borders(Borders::NONE)
        .title(title)
        .fg(LABEL_COLOR)
}
