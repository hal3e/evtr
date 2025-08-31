use crossterm::event::{Event, EventStream as TermEventStream, KeyCode, KeyEventKind};
use evdev::{AbsoluteAxisCode, Device, EventStream, EventType, RelativeAxisCode};
use futures::StreamExt;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize, palette::tailwind},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph, Widget},
};
use std::collections::HashMap;
use tokio::select;

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
struct RelativeAxisInfo {
    code: u16,
    name: String,
}

pub struct DeviceMonitor {
    device_stream: EventStream,
    axis_values: HashMap<u16, i32>,
    axes: Vec<AxisInfo>,
    relative_axis_values: HashMap<u16, i32>,
    relative_axes: Vec<RelativeAxisInfo>,
    button_states: HashMap<u16, bool>,
    buttons: Vec<ButtonInfo>,
}

impl DeviceMonitor {
    fn new(device: Device) -> Result<Self, Box<dyn std::error::Error>> {
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

        // Get all available relative axes from the device
        let mut relative_axes = Vec::new();
        let mut initial_relative_axis_values = HashMap::new();
        if let Some(rel_axes) = device.supported_relative_axes() {
            for axis_type in rel_axes.iter() {
                let code = axis_type.0;
                let name = format!("{:?}", RelativeAxisCode(code));

                relative_axes.push(RelativeAxisInfo { code, name });
                initial_relative_axis_values.insert(code, 0);
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

        // Create event stream from device
        let device_stream = device.into_event_stream()?;

        Ok(Self {
            device_stream,
            axis_values: initial_axis_values,
            axes,
            relative_axis_values: initial_relative_axis_values,
            relative_axes,
            button_states: initial_button_states,
            buttons,
        })
    }

    pub async fn monitor_device(
        terminal: &mut DefaultTerminal,
        device: Device,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut monitor = Self::new(device)?;
        let mut term_events = TermEventStream::new();

        loop {
            // Draw the UI
            terminal.draw(|frame| {
                monitor.render(frame.area(), frame.buffer_mut());
            })?;

            // Use select! to wait for either terminal or device events
            select! {
                // Terminal events
                Some(Ok(event)) = term_events.next() => {
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    return Ok(true); // Go back to device selection
                                }
                                KeyCode::Char('r') => {
                                    monitor.reset_relative_axes();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // Device events
                Ok(ev) = monitor.device_stream.next_event() => {
                    monitor.handle_device_event(ev);
                }
            }
        }
    }

    fn handle_device_event(&mut self, event: evdev::InputEvent) {
        match event.event_type() {
            EventType::ABSOLUTE => {
                let code = event.code();
                let value = event.value();
                self.axis_values.insert(code, value);
            }
            EventType::RELATIVE => {
                let code = event.code();
                let value = event.value();
                // Accumulate relative values
                let current = self.relative_axis_values.get(&code).copied().unwrap_or(0);
                let new_value = current.saturating_add(value);
                self.relative_axis_values.insert(code, new_value);
            }
            EventType::KEY => {
                let code = event.code();
                let pressed = event.value() != 0;
                self.button_states.insert(code, pressed);
            }
            _ => {}
        }
    }

    fn reset_relative_axes(&mut self) {
        for value in self.relative_axis_values.values_mut() {
            *value = 0;
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

    fn render(&self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Ratio};

        let button_rows = if self.buttons.is_empty() {
            0
        } else {
            // Calculate rows needed for buttons (assuming 6 buttons per row)
            self.buttons.len().div_ceil(6)
        };

        // Calculate space needed for relative axes (3 lines per axis)
        let relative_axes_height = if self.relative_axes.is_empty() {
            0
        } else {
            self.relative_axes.len() as u16 * 3 + 1
        };

        let layout = Layout::vertical([
            Length(2),                          // header
            Length(self.axes.len() as u16 * 4), // absolute axes gauges
            Length(relative_axes_height),       // relative axes
            Length(button_rows as u16 * 3 + 2), // buttons + spacing
            Length(1),                          // footer
        ]);
        let [
            header_area,
            gauge_area,
            relative_area,
            button_area,
            footer_area,
        ] = layout.areas(area);

        self.render_header(header_area, buf);
        self.render_footer(footer_area, buf);

        // Render gauges for absolute axes
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

        // Render relative axes
        if !self.relative_axes.is_empty() {
            self.render_relative_axes(relative_area, buf);
        }

        // Render buttons
        if !self.buttons.is_empty() {
            self.render_buttons(button_area, buf);
        }
    }

    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Device Monitor - Live Input Values")
            .bold()
            .alignment(Alignment::Center)
            .fg(LABEL_COLOR)
            .render(area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Press 'q' or ESC to go back | 'r' to reset relative axes")
            .alignment(Alignment::Center)
            .fg(LABEL_COLOR)
            .bold()
            .render(area, buf);
    }

    fn render_gauge(&self, axis_code: u16, title: &str, area: Rect, buf: &mut Buffer) {
        let (value, normalized) = self.get_axis_value(axis_code);

        Gauge::default()
            .block(title_block(title))
            .gauge_style(GAUGE_COLOR)
            .ratio(normalized)
            .label(format!("{value}"))
            .render(area, buf);
    }

    fn render_relative_axes(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return;
        }

        // Title for relative axes section
        let title_area = Rect::new(area.x, area.y, area.width, 1);
        Paragraph::new("Relative Axes")
            .style(Style::default().fg(LABEL_COLOR).bold())
            .alignment(Alignment::Center)
            .render(title_area, buf);

        // Render each relative axis as a row with name and value
        let content_area = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );

        for (i, axis_info) in self.relative_axes.iter().enumerate() {
            let y_offset = i as u16 * 3;
            if y_offset + 2 >= content_area.height {
                break; // No more space
            }

            let axis_area = Rect::new(
                content_area.x + 2,
                content_area.y + y_offset,
                content_area.width.saturating_sub(4),
                3,
            );

            let value = self
                .relative_axis_values
                .get(&axis_info.code)
                .copied()
                .unwrap_or(0);

            // Rolling gauge implementation
            // Use a fixed range for visualization
            const DISPLAY_RANGE: i32 = 500; // -250 to +250
            const HALF_RANGE: i32 = DISPLAY_RANGE / 2;

            // Wrap the value within the display range
            let wrapped_value = if value == 0 {
                0
            } else {
                // Use modulo to wrap the value, keeping sign information
                let wrapped = value % DISPLAY_RANGE;
                if wrapped > HALF_RANGE {
                    wrapped - DISPLAY_RANGE
                } else if wrapped < -HALF_RANGE {
                    wrapped + DISPLAY_RANGE
                } else {
                    wrapped
                }
            };

            // Normalize wrapped value to 0.0-1.0 range for gauge
            let normalized = ((wrapped_value + HALF_RANGE) as f64) / DISPLAY_RANGE as f64;

            // Show axis name with actual value
            let label = format!("{}: {}", axis_info.name, value);

            Gauge::default()
                .block(Block::default().borders(Borders::ALL))
                .gauge_style(GAUGE_COLOR)
                .ratio(normalized.clamp(0.0, 1.0))
                .label(label)
                .render(axis_area, buf);
        }
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

        // Clean up button name
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

fn title_block(title: &str) -> Block<'_> {
    let title = Line::from(title).centered();

    Block::new()
        .borders(Borders::NONE)
        .title(title)
        .fg(LABEL_COLOR)
}
