use crossterm::event::{Event, EventStream as TermEventStream, KeyCode, KeyEventKind};
use evdev::{AbsoluteAxisCode, Device, EventStream, EventType, InputEvent, RelativeAxisCode};
use futures::StreamExt;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Gauge, Paragraph, Widget},
};
use std::collections::BTreeMap;
use tokio::select;

use crate::device::DeviceInfo;

mod config {
    use super::*;
    use ratatui::style::palette::tailwind;

    pub const BUTTONS_PER_ROW: usize = 6;
    pub const BUTTON_HEIGHT: u16 = 3;
    pub const RELATIVE_DISPLAY_RANGE: i32 = 1000; // -500 to +500 range
    pub const DEFAULT_AXIS_RANGE: (i32, i32) = (-32768, 32767);
    pub const BAR_HEIGHTS: [u16; 3] = [5, 3, 1];
    pub const AXIS_LABEL_MAX: u16 = 20; // max chars allocated to axis label
    pub const AXIS_GAP: u16 = 1; // vertical gap between axis bars
    pub const REL_SECTION_GAP: u16 = 1; // spacer before relative section
    pub const BTN_SECTION_TOP_PADDING: u16 = 1; // top padding inside button grid area
    pub const BTN_SECTION_VERT_PADDING: u16 = 2; // total vertical padding used for button section sizing
    pub const BTN_COL_GAP: u16 = 1; // column gap inside button grid
    pub const PAGE_SCROLL_STEPS: usize = 10; // page up/down step count
    pub const AXIS_MIN_WIDTH: u16 = 20; // minimum width to render axis/gauge
    pub const LABEL_GAUGE_GAP: u16 = 1; // horizontal gap between label and gauge

    pub fn style_label() -> Style {
        Style::new().fg(tailwind::SLATE.c200)
    }

    pub fn style_header() -> Style {
        Style::new().fg(tailwind::SLATE.c200).bold()
    }

    pub fn style_gauge() -> Style {
        Style::new().fg(tailwind::BLUE.c400)
    }
    pub const COLOR_BUTTON_PRESSED: Color = tailwind::RED.c400;
}

mod ui {
    pub fn truncate_ascii(text: &str, max_len: usize) -> String {
        if text.len() > max_len {
            let keep = max_len.saturating_sub(3);
            format!("{}...", &text[..keep])
        } else {
            text.to_string()
        }
    }
}

#[derive(Debug, Clone)]
enum InputKind {
    Absolute { min: i32, max: i32, value: i32 },
    Relative(i32),
    Button(bool),
}

impl InputKind {
    fn normalized(&self) -> f64 {
        match *self {
            Self::Absolute { min, max, value } => math::normalize_range(value, min, max),
            Self::Relative(value) => math::normalize_wrapped(value, config::RELATIVE_DISPLAY_RANGE),
            Self::Button(pressed) => {
                if pressed {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    fn display_label(&self) -> String {
        match self {
            Self::Absolute { value, .. } => value.to_string(),
            Self::Relative(value) => {
                // Display the wrapped value for clarity
                math::wrapped_value(*value, config::RELATIVE_DISPLAY_RANGE).to_string()
            }
            Self::Button(true) => button_label(true).to_string(),
            Self::Button(false) => button_label(false).to_string(),
        }
    }

    fn update(&mut self, event: &InputEvent) {
        let value = event.value();
        match (self, event.event_type()) {
            (Self::Absolute { value: v, .. }, EventType::ABSOLUTE) => *v = value,
            (Self::Relative(v), EventType::RELATIVE) => {
                // Just accumulate the value, let it grow/shrink freely
                *v += value;
            }
            (Self::Button(pressed), EventType::KEY) => *pressed = value != 0,
            _ => {}
        }
    }
}

fn button_label(pressed: bool) -> &'static str {
    if pressed { "ON" } else { "OFF" }
}

#[derive(Debug, Clone)]
struct DeviceInput {
    name: String,
    input_type: InputKind,
}

type InputsVec<'a> = Vec<&'a DeviceInput>;
type InputSlice<'a> = &'a [&'a DeviceInput];

mod math {
    pub fn normalize_range(value: i32, min: i32, max: i32) -> f64 {
        let range = (max - min) as f64;
        if range > 0.0 {
            ((value - min) as f64 / range).clamp(0.0, 1.0)
        } else {
            0.5
        }
    }

    pub fn wrapped_value(value: i32, range: i32) -> i32 {
        let half_range = range / 2;
        let mut wrapped = value % range;
        if wrapped > half_range {
            wrapped -= range;
        } else if wrapped < -half_range {
            wrapped += range;
        }
        wrapped
    }

    pub fn normalize_wrapped(value: i32, range: i32) -> f64 {
        let half_range = range / 2;
        let wrapped = wrapped_value(value, range);
        ((wrapped + half_range) as f64 / range as f64).clamp(0.0, 1.0)
    }
}

fn visible_window(total: usize, offset: usize, capacity: usize) -> (usize, usize) {
    if total == 0 || capacity == 0 {
        return (0, 0);
    }
    let start = offset.min(total.saturating_sub(1));
    let remaining = total.saturating_sub(start);
    let count = remaining.min(capacity);
    (start, count)
}

struct InputCollection {
    inputs: BTreeMap<u16, DeviceInput>,
}

impl InputCollection {
    fn from_device(device: &Device) -> Self {
        let mut inputs = BTreeMap::new();

        // Collect absolute axes
        if let Some(axes) = device.supported_absolute_axes() {
            let abs_state = device.get_abs_state().ok();
            for axis in axes.iter() {
                let code = axis.0;
                let (min, max, value) = abs_state
                    .as_ref()
                    .and_then(|s| s.get(code as usize))
                    .map(|info| (info.minimum, info.maximum, info.value))
                    .unwrap_or((
                        config::DEFAULT_AXIS_RANGE.0,
                        config::DEFAULT_AXIS_RANGE.1,
                        0,
                    ));

                inputs.insert(
                    code,
                    DeviceInput {
                        name: format!("{:?}", AbsoluteAxisCode(code)),
                        input_type: InputKind::Absolute { min, max, value },
                    },
                );
            }
        }

        // Collect relative axes
        if let Some(axes) = device.supported_relative_axes() {
            for axis in axes.iter() {
                let code = axis.0;
                inputs.insert(
                    code,
                    DeviceInput {
                        name: format!("{:?}", RelativeAxisCode(code)),
                        input_type: InputKind::Relative(0),
                    },
                );
            }
        }

        // Collect buttons
        if let Some(keys) = device.supported_keys() {
            for key in keys.iter() {
                let code = key.0;
                inputs.insert(
                    code,
                    DeviceInput {
                        name: format!("{key:?}"),
                        input_type: InputKind::Button(false),
                    },
                );
            }
        }

        Self { inputs }
    }

    fn handle_event(&mut self, event: &InputEvent) {
        if let Some(input) = self.inputs.get_mut(&event.code()) {
            input.input_type.update(event);
        }
    }

    fn reset_relative_axes(&mut self) {
        for input in self.inputs.values_mut() {
            if let InputKind::Relative(v) = &mut input.input_type {
                *v = 0;
            }
        }
    }

    fn iter_absolute(&self) -> impl Iterator<Item = &DeviceInput> {
        self.inputs
            .values()
            .filter(|input| matches!(input.input_type, InputKind::Absolute { .. }))
    }

    fn iter_relative(&self) -> impl Iterator<Item = &DeviceInput> {
        self.inputs
            .values()
            .filter(|input| matches!(input.input_type, InputKind::Relative(_)))
    }

    fn iter_buttons(&self) -> impl Iterator<Item = &DeviceInput> {
        self.inputs
            .values()
            .filter(|input| matches!(input.input_type, InputKind::Button(_)))
    }
}

// ====== Rendering Components ======
struct AxisRenderer;

impl AxisRenderer {
    fn split_label_gauge(area: Rect) -> (Rect, Rect) {
        let label_width = config::AXIS_LABEL_MAX.min(area.width / 3);
        let gauge_width = area
            .width
            .saturating_sub(label_width + config::LABEL_GAUGE_GAP);
        let [label_area, gauge_area] = Layout::horizontal([
            Constraint::Length(label_width),
            Constraint::Length(gauge_width),
        ])
        .areas(area);

        let label_y = if area.height > 1 {
            label_area.y + (area.height / 2)
        } else {
            label_area.y
        };

        let label_rect = Rect::new(label_area.x, label_y, label_area.width, 1);
        (label_rect, gauge_area)
    }

    fn render_axes_with_scroll(
        inputs: InputSlice,
        area: Rect,
        scroll_offset: usize,
        buf: &mut Buffer,
    ) {
        if inputs.is_empty() || area.height == 0 {
            return;
        }

        // Calculate optimal bar height for visible items
        let num_items = inputs.len();
        let bar_height = Self::calculate_bar_height(area.height, num_items);
        let item_height = bar_height + config::AXIS_GAP; // bar height + gap

        // Calculate how many items can fit
        let max_visible = (area.height / item_height) as usize;
        let (start, count) = visible_window(num_items, scroll_offset, max_visible);
        for (i, input) in inputs[start..start + count].iter().enumerate() {
            let y = area.y + (i as u16 * item_height);
            if y + bar_height > area.y + area.height {
                break;
            }

            let item_area = Rect::new(area.x, y, area.width, bar_height);
            Self::render_axis_item(input, item_area, buf);
        }
    }

    fn calculate_bar_height(available_height: u16, num_items: usize) -> u16 {
        if num_items == 0 {
            return 1;
        }

        // Try odd numbers in descending order: 5, 3, 1 (removed 7)
        for height in &config::BAR_HEIGHTS {
            let total_needed = (height + config::AXIS_GAP) * num_items as u16; // include gap
            if total_needed <= available_height {
                return *height;
            }
        }

        1 // Default to 1 if nothing fits
    }

    fn render_axis_item(input: &DeviceInput, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || area.width < config::AXIS_MIN_WIDTH {
            return;
        }

        let (label_rect, gauge_area) = Self::split_label_gauge(area);
        if gauge_area.width == 0 {
            return;
        }
        let truncated_name = ui::truncate_ascii(&input.name, label_rect.width as usize);

        Paragraph::new(truncated_name)
            .style(config::style_label())
            .alignment(Alignment::Left)
            .render(label_rect, buf);

        // Render gauge on the right with value
        let value_str = input.input_type.display_label();
        let ratio = input.input_type.normalized();

        Gauge::default()
            .gauge_style(config::style_gauge())
            .ratio(ratio)
            .label(value_str)
            .render(gauge_area, buf);
    }
}

struct ButtonGrid;

struct GridMetrics {
    button_width: u16,
    max_rows: usize,
}

impl ButtonGrid {
    fn metrics(area: Rect) -> GridMetrics {
        let button_width = area.width / config::BUTTONS_PER_ROW as u16;
        let max_rows = ((area.height.saturating_sub(config::BTN_SECTION_VERT_PADDING))
            / config::BUTTON_HEIGHT) as usize;
        GridMetrics {
            button_width,
            max_rows,
        }
    }

    fn render_with_scroll(
        buttons: InputSlice,
        area: Rect,
        scroll_row_offset: usize,
        buf: &mut Buffer,
    ) {
        if buttons.is_empty() || area.height < config::BUTTON_HEIGHT {
            return;
        }

        let metrics = Self::metrics(area);
        if metrics.max_rows == 0 {
            return;
        }

        // Calculate which buttons to show based on row offset
        let start_button = scroll_row_offset * config::BUTTONS_PER_ROW;
        let max_visible_buttons = metrics.max_rows * config::BUTTONS_PER_ROW;
        let (start, count) = visible_window(buttons.len(), start_button, max_visible_buttons);

        for i in 0..count {
            let idx = start + i;
            let (row, col) = Self::grid_position(i);
            let button_area = Self::calculate_button_area(area, row, col, metrics.button_width);
            Self::render_button(button_area, buttons[idx], buf);
        }
    }

    fn grid_position(index: usize) -> (usize, usize) {
        (
            index / config::BUTTONS_PER_ROW,
            index % config::BUTTONS_PER_ROW,
        )
    }

    fn calculate_button_area(area: Rect, row: usize, col: usize, button_width: u16) -> Rect {
        Rect::new(
            area.x + (col as u16 * button_width),
            area.y + config::BTN_SECTION_TOP_PADDING + (row as u16 * config::BUTTON_HEIGHT),
            button_width.saturating_sub(config::BTN_COL_GAP),
            config::BUTTON_HEIGHT,
        )
    }

    fn render_button(area: Rect, input: &DeviceInput, buf: &mut Buffer) {
        let pressed = matches!(input.input_type, InputKind::Button(true));

        let block = Block::default().borders(Borders::ALL).bg(if pressed {
            config::COLOR_BUTTON_PRESSED
        } else {
            Color::default()
        });

        let text = Self::truncate_text(&input.name, area.width.saturating_sub(2) as usize);

        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .style(config::style_label())
            .render(area, buf);
    }

    fn truncate_text(text: &str, max_len: usize) -> String {
        ui::truncate_ascii(text, max_len)
    }
}

// ====== Main Monitor ======
pub struct DeviceMonitor {
    device_stream: EventStream,
    inputs: InputCollection,
    scroll_offset: usize,
    last_content_area_height: u16,
    identifier: String,
}

struct Counts {
    abs: usize,
    rel: usize,
    btn: usize,
}

impl Counts {
    fn total_axes(&self) -> usize {
        self.abs + self.rel
    }
    fn btn_rows(&self) -> usize {
        self.btn.div_ceil(config::BUTTONS_PER_ROW)
    }
}

impl DeviceMonitor {
    fn new(
        DeviceInfo { device, identifier }: DeviceInfo,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let inputs = InputCollection::from_device(&device);

        let device_stream = device.into_event_stream()?;
        Ok(Self {
            device_stream,
            inputs,
            scroll_offset: 0,
            last_content_area_height: 40, // Default estimate
            identifier,
        })
    }

    pub async fn run(
        terminal: &mut DefaultTerminal,
        device_info: DeviceInfo,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut monitor = Self::new(device_info)?;
        let mut term_events = TermEventStream::new();

        loop {
            terminal.draw(|frame| monitor.render(frame.area(), frame.buffer_mut()))?;

            select! {
                event = term_events.next() => {
                    if let Some(Ok(Event::Key(key))) = event {
                        if key.kind == KeyEventKind::Press {
                            match monitor.handle_key(key.code) {
                                Command::Quit => return Ok(true),
                                Command::Reset => monitor.inputs.reset_relative_axes(),
                                Command::Scroll(dir) => {
                                    if dir < 0 { monitor.scroll_up(); } else { monitor.scroll_down(); }
                                }
                                Command::Page(dir) => monitor.scroll_page(dir),
                                Command::Home => monitor.scroll_offset = 0,
                                Command::None => {}
                            }
                        }
                    } else if let Some(Err(e)) = event {
                        return Err(Box::new(e));
                    }
                }
                event = monitor.device_stream.next_event() => {
                    monitor.inputs.handle_event(&event?);
                }
            }
        }
    }

    fn scroll_page(&mut self, dir: i32) {
        let counts = self.compute_counts();
        let steps = config::PAGE_SCROLL_STEPS;
        for _ in 0..steps {
            self.scroll_step(&counts, dir);
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let [header, content, footer] = main_layout(area);

        Paragraph::new(self.identifier.as_str())
            .style(config::style_header())
            .alignment(Alignment::Center)
            .render(header, buf);

        // Store content height for scroll overflow calculation
        self.last_content_area_height = content.height;

        // Content
        self.render_content(content, buf);

        // Footer - show scroll info only if scrolling is needed
        let counts = self.compute_counts();
        let overflow = self.overflow_from_counts(&counts);
        let footer_text = self.build_footer_text(&counts, overflow);

        Paragraph::new(footer_text)
            .style(config::style_header())
            .alignment(Alignment::Center)
            .render(footer, buf);
    }

    fn render_content(&self, area: Rect, buf: &mut Buffer) {
        let counts = self.compute_counts();

        // Prepare filtered views once per render
        let abs_inputs: InputsVec = self.inputs.iter_absolute().collect();
        let rel_inputs: InputsVec = self.inputs.iter_relative().collect();
        let btn_inputs: InputsVec = self.inputs.iter_buttons().collect();

        // Calculate minimum space needed for each section
        let total_axes = counts.total_axes();
        if total_axes == 0 {
            // No axes, just render buttons at top
            if counts.btn > 0 {
                let btn_rows = counts.btn_rows();
                let btn_height =
                    (btn_rows as u16 * config::BUTTON_HEIGHT) + config::BTN_SECTION_VERT_PADDING;
                let btn_area = Rect::new(area.x, area.y, area.width, btn_height.min(area.height));
                self.render_buttons(btn_area, &btn_inputs, total_axes, buf);
            }
            return;
        }

        let sizer = SectionSizer::new(area, &counts, total_axes);

        Self::render_axes_sections(
            self,
            area,
            sizer.axes_height,
            counts.abs,
            counts.rel,
            &abs_inputs,
            &rel_inputs,
            buf,
        );

        if let Some(btn_area) = sizer.btn_area {
            self.render_buttons(btn_area, &btn_inputs, total_axes, buf);
        }
    }

    fn calculate_optimal_axes_height(
        min_height: u16,
        available_height: u16,
        total_axes: usize,
    ) -> u16 {
        if total_axes == 0 || available_height <= min_height {
            return min_height;
        }

        // Try to fit axes with expanded heights (5, 3, 1) but only use what we actually need
        let rel_section_gap = config::REL_SECTION_GAP; // Assume there might be a relative section

        // Try each height to see what fits
        for &bar_height in &config::BAR_HEIGHTS {
            let total_needed =
                (total_axes as u16 * (bar_height + config::AXIS_GAP)) + rel_section_gap;
            if total_needed <= available_height {
                return total_needed.min(available_height);
            }
        }

        min_height
    }

    fn render_axes_sections(
        device_monitor: &DeviceMonitor,
        area: Rect,
        axes_height: u16,
        abs_count: usize,
        rel_count: usize,
        abs_inputs: InputSlice,
        rel_inputs: InputSlice,
        buf: &mut Buffer,
    ) {
        let axes_area = Rect::new(area.x, area.y, area.width, axes_height);

        if abs_count > 0 && rel_count > 0 {
            // Both types present - split proportionally
            let total_axes = abs_count + rel_count;
            let rel_section_gap = config::REL_SECTION_GAP; // Gap for relative section
            let available_for_content = axes_height.saturating_sub(rel_section_gap);

            let abs_portion = (available_for_content * abs_count as u16) / total_axes as u16;
            let rel_portion = available_for_content.saturating_sub(abs_portion) + rel_section_gap;

            let abs_area = Rect::new(axes_area.x, axes_area.y, axes_area.width, abs_portion);
            let rel_area = Rect::new(
                axes_area.x,
                axes_area.y + abs_portion,
                axes_area.width,
                rel_portion,
            );

            let (abs_off, rel_off) = device_monitor.axis_offsets(abs_count);
            AxisRenderer::render_axes_with_scroll(abs_inputs, abs_area, abs_off, buf);
            AxisRenderer::render_axes_with_scroll(rel_inputs, rel_area, rel_off, buf);
        } else if abs_count > 0 {
            // Only absolute axes
            let (abs_off, _) = device_monitor.axis_offsets(abs_count);
            AxisRenderer::render_axes_with_scroll(abs_inputs, axes_area, abs_off, buf);
        } else {
            // Only relative axes
            let (_, rel_off) = device_monitor.axis_offsets(abs_count);
            AxisRenderer::render_axes_with_scroll(rel_inputs, axes_area, rel_off, buf);
        }
    }

    fn scroll_up(&mut self) {
        // Only allow scrolling if we have overflow
        if !self.has_overflow() {
            return;
        }
        let counts = self.compute_counts();
        self.scroll_step(&counts, -1);
    }

    fn scroll_down(&mut self) {
        // Only allow scrolling if we have overflow
        if !self.has_overflow() {
            return;
        }
        let counts = self.compute_counts();
        self.scroll_step(&counts, 1);
    }

    fn handle_key(&mut self, code: KeyCode) -> Command {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => Command::Quit,
            KeyCode::Char('r') => Command::Reset,
            KeyCode::Up | KeyCode::Char('k') => Command::Scroll(-1),
            KeyCode::Down | KeyCode::Char('j') => Command::Scroll(1),
            KeyCode::PageUp => Command::Page(-1),
            KeyCode::PageDown => Command::Page(1),
            KeyCode::Home => Command::Home,
            _ => Command::None,
        }
    }

    fn has_overflow(&self) -> bool {
        let counts = self.compute_counts();
        self.overflow_from_counts(&counts)
    }

    fn compute_counts(&self) -> Counts {
        Counts {
            abs: self.inputs.iter_absolute().count(),
            rel: self.inputs.iter_relative().count(),
            btn: self.inputs.iter_buttons().count(),
        }
    }

    fn build_footer_text(&self, counts: &Counts, overflow: bool) -> String {
        let has_relative = counts.rel > 0;
        if overflow {
            let total_items = counts.abs + counts.rel + counts.btn;
            let prefix = Self::footer_prefix(has_relative, true);
            format!(
                "{} Items: {} | Offset: {}",
                prefix, total_items, self.scroll_offset
            )
        } else {
            Self::footer_prefix(has_relative, false).to_string()
        }
    }

    fn scroll_step(&mut self, counts: &Counts, direction: i32) {
        let total_axes = counts.total_axes();
        if direction < 0 {
            if self.scroll_offset < total_axes {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            } else {
                let button_offset = self.scroll_offset - total_axes;
                let current_button_row = button_offset / config::BUTTONS_PER_ROW;
                if current_button_row > 0 {
                    self.scroll_offset =
                        total_axes + ((current_button_row - 1) * config::BUTTONS_PER_ROW);
                } else if button_offset > 0 {
                    self.scroll_offset = total_axes.saturating_sub(1);
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
            }
        } else if direction > 0 {
            if counts.btn == 0 && self.scroll_offset >= total_axes.saturating_sub(1) {
                return;
            }
            if self.scroll_offset < total_axes {
                if self.scroll_offset + 1 < total_axes {
                    self.scroll_offset += 1;
                } else if counts.btn > 0 {
                    self.scroll_offset = total_axes;
                } else {
                    self.scroll_offset = total_axes.saturating_sub(1);
                }
            } else {
                let button_offset = self.scroll_offset - total_axes;
                let current_button_row = button_offset / config::BUTTONS_PER_ROW;
                let total_button_rows = counts.btn_rows();
                if current_button_row + 1 < total_button_rows {
                    self.scroll_offset =
                        total_axes + ((current_button_row + 1) * config::BUTTONS_PER_ROW);
                }
            }
        }
    }

    fn button_row_offset(&self, total_axes: usize) -> usize {
        let button_scroll_offset = self.scroll_offset.saturating_sub(total_axes);
        button_scroll_offset / config::BUTTONS_PER_ROW
    }

    fn render_buttons(
        &self,
        btn_area: Rect,
        btn_inputs: InputSlice,
        total_axes: usize,
        buf: &mut Buffer,
    ) {
        let row_offset = self.button_row_offset(total_axes);
        ButtonGrid::render_with_scroll(btn_inputs, btn_area, row_offset, buf);
    }

    fn overflow_from_counts(&self, counts: &Counts) -> bool {
        let min_axis_height = (counts.total_axes() as u16) * (1 + config::AXIS_GAP);
        let rel_title_height = if counts.rel > 0 {
            config::REL_SECTION_GAP
        } else {
            0
        };
        let btn_rows = counts.btn_rows();
        let btn_height = if btn_rows > 0 {
            (btn_rows as u16 * config::BUTTON_HEIGHT) + config::BTN_SECTION_VERT_PADDING
        } else {
            0
        };
        let total_min_height = min_axis_height + rel_title_height + btn_height;
        total_min_height > self.last_content_area_height
    }

    fn footer_prefix(has_relative: bool, overflow: bool) -> &'static str {
        match (overflow, has_relative) {
            (true, true) => "'q'/ESC: exit | 'r': reset | ↑/↓ or j/k: scroll | PgUp/PgDn: fast |",
            (true, false) => "'q'/ESC: exit | ↑/↓ or j/k: scroll | PgUp/PgDn: fast |",
            (false, true) => "'q'/ESC: exit | 'r': reset relative axes",
            (false, false) => "'q'/ESC: exit",
        }
    }

    fn axis_offsets(&self, abs_count: usize) -> (usize, usize) {
        (
            self.scroll_offset,
            self.scroll_offset.saturating_sub(abs_count),
        )
    }
}

// ====== Layout Calculation ======
fn main_layout(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area)
}

struct SectionSizer {
    axes_height: u16,
    btn_area: Option<Rect>,
}

impl SectionSizer {
    fn new(area: Rect, counts: &Counts, total_axes: usize) -> Self {
        if total_axes == 0 {
            // No axes; only buttons take space at top
            let btn_rows = counts.btn_rows();
            let btn_height = if btn_rows == 0 {
                0
            } else {
                (btn_rows as u16 * config::BUTTON_HEIGHT) + config::BTN_SECTION_VERT_PADDING
            };
            let h = btn_height.min(area.height);
            return Self {
                axes_height: 0,
                btn_area: if h > 0 {
                    Some(Rect::new(area.x, area.y, area.width, h))
                } else {
                    None
                },
            };
        }

        let rel_title_height = if counts.rel > 0 {
            config::REL_SECTION_GAP
        } else {
            0
        };
        let min_axes_height = (total_axes as u16 * (1 + config::AXIS_GAP)) + rel_title_height;

        let btn_rows = counts.btn_rows();
        let btn_height = if btn_rows == 0 {
            0
        } else {
            (btn_rows as u16 * config::BUTTON_HEIGHT) + config::BTN_SECTION_VERT_PADDING
        };

        let min_total_needed = min_axes_height + btn_height;
        if min_total_needed > area.height {
            // Not enough space: squeeze axes, reduce buttons if needed
            let axes_height = area.height.saturating_sub(btn_height);
            let actual_btn_h = btn_height.min(area.height.saturating_sub(axes_height));
            let btn_area = if actual_btn_h > 0 {
                Some(Rect::new(
                    area.x,
                    area.y + axes_height,
                    area.width,
                    actual_btn_h,
                ))
            } else {
                None
            };
            Self {
                axes_height,
                btn_area,
            }
        } else {
            // Enough space: expand axes up to optimal; fixed button height
            let axes_height = DeviceMonitor::calculate_optimal_axes_height(
                min_axes_height,
                area.height - btn_height,
                total_axes,
            );
            let btn_area = if btn_height > 0 {
                Some(Rect::new(
                    area.x,
                    area.y + axes_height,
                    area.width,
                    btn_height,
                ))
            } else {
                None
            };
            Self {
                axes_height,
                btn_area,
            }
        }
    }
}
enum Command {
    Quit,
    Reset,
    Scroll(i32),
    Page(i32),
    Home,
    None,
}
