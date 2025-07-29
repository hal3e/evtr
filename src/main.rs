mod devices;

use std::{collections::HashMap, time::Duration};

use color_eyre::Result;
use evdev::{AbsoluteAxisCode, Device, EventType};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Stylize, palette::tailwind},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph, Widget},
};

use crate::devices::select_device;

const GAUGE_COLOR: Color = tailwind::BLUE.c500;
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
struct App {
    state: AppState,
    device: Option<Device>,
    axis_values: HashMap<u16, i32>,
    axes: Vec<AxisInfo>,
    button_states: HashMap<u16, bool>,
    buttons: Vec<ButtonInfo>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum AppState {
    #[default]
    DeviceSelection,
    Running,
    BackToSelection,
    Quitting,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    loop {
        let device = match select_device() {
            Ok(device) => device,
            Err(e) => {
                eprintln!("Error selecting device: {}", e);
                break;
            }
        };

        let mut device = device;
        device.set_nonblocking(true).map_err(|e| {
            eprintln!("Error setting non-blocking mode: {}", e);
            color_eyre::eyre::eyre!("Failed to set non-blocking mode: {}", e)
        })?;

        let app = create_app_with_device(device)?;
        let terminal = ratatui::init();

        let app_result = app.run(terminal);
        ratatui::restore();

        match app_result {
            Ok(should_continue) => {
                if !should_continue {
                    break;
                }
                // Continue loop to show device selection again
            }
            Err(e) => {
                eprintln!("App error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn create_app_with_device(device: evdev::Device) -> Result<App> {
    // Get all available axes from the device
    let mut axes = Vec::new();
    let mut initial_axis_values = HashMap::new();
    if let Some(abs_axes) = device.supported_absolute_axes() {
        for axis_type in abs_axes.iter() {
            let code = axis_type.0;
            let name = format!("{:?}", AbsoluteAxisCode(code));

            // Try to get axis info including min/max values
            let (min, max, center) = if let Ok(abs_state) = device.get_abs_state() {
                if code < abs_state.len() as u16 {
                    let info = &abs_state[code as usize];
                    let min = info.minimum;
                    let max = info.maximum;
                    let center = (min + max) / 2;
                    (min, max, center)
                } else {
                    // Default range if index out of bounds
                    (-32768, 32767, -1)
                }
            } else {
                // Default range if no state available
                (-32768, 32767, -1)
            };

            axes.push(AxisInfo {
                code,
                name,
                min,
                max,
            });
            initial_axis_values.insert(code, center);
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
                name: format!("{:?}", key),
            });
            initial_button_states.insert(code, false);
        }
    }

    Ok(App {
        state: AppState::Running,
        device: Some(device),
        axis_values: initial_axis_values,
        axes,
        button_states: initial_button_states,
        buttons,
    })
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<bool> {
        while self.state != AppState::Quitting && self.state != AppState::BackToSelection {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            self.handle_events()?;
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

    fn handle_events(&mut self) -> Result<()> {
        let timeout = Duration::from_secs_f32(1.0 / 20.0);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn quit(&mut self) {
        self.state = AppState::BackToSelection;
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
        use Constraint::{Length, Ratio};

        let button_rows = if self.buttons.is_empty() {
            0
        } else {
            // Calculate rows needed for buttons (assuming 6 buttons per row)
            (self.buttons.len() + 5) / 6
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
            .label(format!("{}", value))
            .render(area, buf);
    }

    fn render_buttons(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 {
            return;
        }

        // Calculate button layout (6 buttons per row)
        let buttons_per_row = 6usize;
        let button_width = area.width / buttons_per_row as u16;
        let rows = (self.buttons.len() + buttons_per_row - 1) / buttons_per_row;

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
            block = block.bg(tailwind::GREEN.c500);
        }

        let fg_color = if is_pressed {
            tailwind::WHITE
        } else {
            LABEL_COLOR
        };

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
            .fg(fg_color)
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
