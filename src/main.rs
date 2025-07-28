mod devices;

use std::{collections::HashMap, time::Duration};

use color_eyre::Result;
use evdev::{Device, EventType};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Stylize, palette::tailwind},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph, Widget},
};

const GAUGE_COLOR: Color = tailwind::BLUE.c500;
const LABEL_COLOR: Color = tailwind::SLATE.c200;

#[derive(Debug)]
struct App {
    state: AppState,
    device: Option<Device>,
    axis_values: HashMap<u16, i32>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum AppState {
    #[default]
    DeviceSelection,
    Running,
    Quitting,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let devices = devices::discover_joystick_devices().map_err(|e| {
        eprintln!("Error discovering joystick devices: {}", e);
        color_eyre::eyre::eyre!("Failed to discover joystick devices: {}", e)
    })?;

    if devices.is_empty() {
        return Err(color_eyre::eyre::eyre!("No joystick devices found!"));
    }

    let device_path = devices::select_device(&devices).map_err(|e| {
        eprintln!("Error selecting device: {}", e);
        color_eyre::eyre::eyre!("Failed to select device: {}", e)
    })?;

    let mut device = Device::open(&device_path).map_err(|e| {
        eprintln!("Error opening device: {}", e);
        color_eyre::eyre::eyre!("Failed to open device: {}", e)
    })?;

    device.set_nonblocking(true).map_err(|e| {
        eprintln!("Error setting non-blocking mode: {}", e);
        color_eyre::eyre::eyre!("Failed to set non-blocking mode: {}", e)
    })?;

    let app = App {
        state: AppState::Running,
        device: Some(device),
        axis_values: HashMap::new(),
    };

    let terminal = ratatui::init();

    let app_result = app.run(terminal);
    ratatui::restore();

    app_result
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.state != AppState::Quitting {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            self.handle_events()?;
            self.update();
        }
        Ok(())
    }

    fn update(&mut self) {
        if self.state == AppState::Quitting {
            return;
        }

        if let Some(ref mut device) = self.device {
            if let Ok(events) = device.fetch_events() {
                for event in events {
                    if event.event_type() == EventType::ABSOLUTE {
                        let code = event.code();
                        let value = event.value();
                        self.axis_values.insert(code, value);
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
        self.state = AppState::Quitting;
    }

    fn get_axis_value(&self, axis_code: u16) -> (i32, f64) {
        let value = self.axis_values.get(&axis_code).copied().unwrap_or(0);
        // Normalize typical joystick range (-32768 to 32767) to 0.0-1.0
        let normalized = ((value + 32768) as f64) / 65535.0;
        (value, normalized.clamp(0.0, 1.0))
    }
}

impl Widget for &App {
    #[allow(clippy::similar_names)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Ratio};

        let layout = Layout::vertical([Length(2), Length(16), Length(1)]);
        let [header_area, gauge_area, footer_area] = layout.areas(area);

        let layout = Layout::vertical([Ratio(1, 4); 4]);
        let [gauge1_area, gauge2_area, gauge3_area, gauge4_area] = layout.areas(gauge_area);

        render_header(header_area, buf);
        render_footer(footer_area, buf);

        self.render_gauge(0, "Left X Axis", gauge1_area, buf);
        self.render_gauge(1, "Left Y Axis", gauge2_area, buf);
        self.render_gauge(3, "Right X Axis", gauge3_area, buf);
        self.render_gauge(4, "Right Y Axis", gauge4_area, buf);
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
}

fn title_block(title: &str) -> Block {
    let title = Line::from(title).centered();

    Block::new()
        .borders(Borders::NONE)
        .title(title)
        .fg(LABEL_COLOR)
}
