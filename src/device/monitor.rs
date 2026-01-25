mod config;
mod controls;
mod layout;
mod math;
mod model;
mod render;
mod touch;
mod theme;
mod ui;

use crossterm::event::{
    Event, EventStream as TermEventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures::StreamExt;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use tokio::select;

use self::{
    controls::Command,
    layout::{axes_layout, box_layout, main_layout},
    model::{InputCollection, InputsVec},
    render::{axis::AxisRenderer, buttons::ButtonGrid, touch::TouchRenderer},
    touch::TouchState,
};
use crate::{
    device::DeviceInfo,
    device::popup::{Popup, render_popup},
    error::{Error, Result},
};

const HELP_POPUP_MIN_WIDTH: u16 = 30;
const HELP_POPUP_MIN_HEIGHT: u16 = 6;
const HELP_POPUP_MAX_WIDTH: u16 = 80;

pub struct DeviceMonitor {
    device_stream: evdev::EventStream,
    inputs: InputCollection,
    identifier: String,
    counts: Counts,
    // Counts adjusted to what is actually renderable in the current layout
    effective_counts: Counts,
    info_popup: DeviceInfoPopup,
    info_visible: bool,
    help_popup: HelpPopup,
    help_visible: bool,
    touch: TouchState,
    focus: Focus,
    axis_scroll: usize,
    button_row_scroll: usize,
    // Max scroll steps across axes (abs then rel).
    axes_scroll_max: usize,
    abs_max_start: usize,
    rel_max_start: usize,
    // Max starting row offset for buttons
    button_row_max_start: usize,
    axes_box_present: bool,
    buttons_box_present: bool,
    axes_overflow: bool,
    buttons_overflow: bool,
}

#[derive(Clone, Copy)]
struct Counts {
    abs: usize,
    rel: usize,
    btn: usize,
}

impl Counts {
    fn total_axes(&self) -> usize {
        self.abs + self.rel
    }

    fn filtered(&self, abs_visible: bool, rel_visible: bool, buttons_visible: bool) -> Self {
        Self {
            abs: if abs_visible { self.abs } else { 0 },
            rel: if rel_visible { self.rel } else { 0 },
            btn: if buttons_visible { self.btn } else { 0 },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Axes,
    Buttons,
}

struct DeviceInfoPopup {
    lines: Vec<String>,
}

impl DeviceInfoPopup {
    fn new(
        driver_version: (u8, u8, u8),
        input_id: evdev::InputId,
        phys: Option<&str>,
    ) -> Self {
        let (major, minor, patch) = driver_version;
        let bus = input_id.bus_type().0;
        let vendor = input_id.vendor();
        let product = input_id.product();
        let version = input_id.version();
        let phys = phys.unwrap_or("n/a");
        let lines = vec![
            format!("Input driver version: {major}.{minor}.{patch}"),
            format!(
                "Input device ID: bus {bus:#x}, vendor {vendor:#x}, product {product:#x}, version {version:#x}"
            ),
            format!("Input device phys: {phys}"),
        ];
        Self { lines }
    }
}

struct HelpPopup {
    lines: Vec<String>,
}

impl HelpPopup {
    fn new() -> Self {
        Self {
            lines: vec![
                "Scroll: Up/Down or j/k, PageUp/PageDown".to_string(),
                "Jump: Home/End or g/G".to_string(),
                "Reset: r (relative axes)".to_string(),
                "Info: i (press i or Esc to close)".to_string(),
                "Focus: Shift+J/Shift+K (when axes and buttons show)".to_string(),
                "Exit: Ctrl-C".to_string(),
                "Help: ? (press ? or Esc to close)".to_string(),
            ],
        }
    }
}

fn axis_offsets_for(
    axis_scroll: usize,
    abs_count: usize,
    rel_count: usize,
    abs_max_start: usize,
    rel_max_start: usize,
) -> (usize, usize) {
    let axes_scroll_max = abs_max_start + rel_max_start;
    let axis_scroll = axis_scroll.min(axes_scroll_max);

    match (abs_count > 0, rel_count > 0) {
        (true, true) => {
            if axis_scroll <= abs_max_start {
                (axis_scroll, 0)
            } else {
                (
                    abs_max_start,
                    (axis_scroll - abs_max_start).min(rel_max_start),
                )
            }
        }
        (true, false) => (axis_scroll.min(abs_max_start), 0),
        (false, true) => (0, axis_scroll.min(rel_max_start)),
        (false, false) => (0, 0),
    }
}

impl DeviceMonitor {
    fn new(DeviceInfo { device, identifier }: DeviceInfo) -> Result<Self> {
        let inputs = InputCollection::from_device(&device);
        let info_popup =
            DeviceInfoPopup::new(device.driver_version(), device.input_id(), device.physical_path());
        let help_popup = HelpPopup::new();
        let touch = TouchState::from_device(&device);
        let device_stream = device
            .into_event_stream()
            .map_err(|err| Error::evdev(format!("open device stream ({identifier})"), err))?;
        let counts = Counts {
            abs: inputs.iter_absolute().count(),
            rel: inputs.iter_relative().count(),
            btn: inputs.iter_buttons().count(),
        };
        let focus = if counts.total_axes() > 0 {
            Focus::Axes
        } else {
            Focus::Buttons
        };
        Ok(Self {
            device_stream,
            inputs,
            identifier,
            effective_counts: counts,
            counts,
            info_popup,
            info_visible: false,
            help_popup,
            help_visible: false,
            touch,
            focus,
            axis_scroll: 0,
            button_row_scroll: 0,
            axes_scroll_max: 0,
            abs_max_start: 0,
            rel_max_start: 0,
            button_row_max_start: 0,
            axes_box_present: false,
            buttons_box_present: false,
            axes_overflow: false,
            buttons_overflow: false,
        })
    }

    pub async fn run(terminal: &mut DefaultTerminal, device_info: DeviceInfo) -> Result<()> {
        let mut monitor = Self::new(device_info)?;
        let mut term_events = TermEventStream::new();

        loop {
            terminal
                .draw(|frame| monitor.render(frame.area(), frame.buffer_mut()))
                .map_err(|err| Error::io("monitor draw", err))?;

            select! {
                event = term_events.next() => {
                    match event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                            if monitor.info_visible {
                                match key.code {
                                    KeyCode::Esc | KeyCode::Char('i') => monitor.toggle_info(),
                                    KeyCode::Char('c')
                                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                    {
                                        return Ok(());
                                    }
                                    _ => {}
                                }
                                continue;
                            }
                            if monitor.help_visible {
                                match key.code {
                                    KeyCode::Esc | KeyCode::Char('?') => monitor.toggle_help(),
                                    KeyCode::Char('c')
                                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                    {
                                        return Ok(());
                                    }
                                    _ => {}
                                }
                                continue;
                            }
                            match monitor.handle_event(key) {
                                Command::Quit => return Ok(()),
                                Command::Reset => monitor.inputs.reset_relative_axes(),
                                Command::Scroll(dir) => monitor.scroll_by(dir),
                                Command::Page(dir) => monitor.scroll_page(dir),
                                Command::Home => monitor.scroll_home(),
                                Command::End => monitor.scroll_end(),
                                Command::FocusNext => monitor.focus_next(),
                                Command::FocusPrev => monitor.focus_prev(),
                                Command::ToggleInfo => monitor.toggle_info(),
                                Command::ToggleHelp => monitor.toggle_help(),
                                Command::None => {}
                            }
                        }
                        Some(Ok(_)) => {}
                        Some(Err(err)) => return Err(Error::terminal("terminal event stream", err)),
                        None => return Err(Error::stream_ended("terminal event stream")),
                    }
                }
                event = monitor.device_stream.next_event() => {
                    let event = event.map_err(|err| {
                        Error::evdev(
                            format!("device event stream ({})", monitor.identifier),
                            err,
                        )
                    })?;
                    monitor.inputs.handle_event(&event);
                    monitor.touch.update(&event);
                }
            }
        }
    }

    fn scroll_by(&mut self, direction: i32) {
        if direction == 0 {
            return;
        }
        match self.focus {
            Focus::Axes => self.scroll_axes(direction),
            Focus::Buttons => self.scroll_buttons(direction),
        }
    }

    fn scroll_page(&mut self, direction: i32) {
        if direction == 0 {
            return;
        }
        for _ in 0..config::PAGE_SCROLL_STEPS {
            self.scroll_by(direction);
        }
    }

    fn scroll_home(&mut self) {
        match self.focus {
            Focus::Axes => self.axis_scroll = 0,
            Focus::Buttons => self.button_row_scroll = 0,
        }
    }

    fn scroll_end(&mut self) {
        match self.focus {
            Focus::Axes => self.axis_scroll = self.axes_scroll_max,
            Focus::Buttons => self.button_row_scroll = self.button_row_max_start,
        }
    }

    fn focus_next(&mut self) {
        if self.focusable() {
            self.focus = match self.focus {
                Focus::Axes => Focus::Buttons,
                Focus::Buttons => Focus::Axes,
            };
        }
    }

    fn focus_prev(&mut self) {
        self.focus_next();
    }

    fn toggle_info(&mut self) {
        self.info_visible = !self.info_visible;
        if self.info_visible {
            self.help_visible = false;
        }
    }

    fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
        if self.help_visible {
            self.info_visible = false;
        }
    }

    fn focusable(&self) -> bool {
        self.axes_box_present && self.buttons_box_present
    }

    fn sync_focus(&mut self) {
        self.focus = match (self.axes_box_present, self.buttons_box_present) {
            (true, true) => self.focus,
            (true, false) => Focus::Axes,
            (false, true) => Focus::Buttons,
            (false, false) => self.focus,
        };
    }

    fn scroll_axes(&mut self, direction: i32) {
        if !self.axes_overflow {
            return;
        }
        if direction < 0 {
            self.axis_scroll = self.axis_scroll.saturating_sub(1);
        } else if direction > 0 {
            self.axis_scroll = (self.axis_scroll + 1).min(self.axes_scroll_max);
        }
    }

    fn scroll_buttons(&mut self, direction: i32) {
        if !self.buttons_overflow {
            return;
        }
        if direction < 0 {
            self.button_row_scroll = self.button_row_scroll.saturating_sub(1);
        } else if direction > 0 {
            self.button_row_scroll = (self.button_row_scroll + 1).min(self.button_row_max_start);
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let [header, content] = main_layout(area);

        Paragraph::new(self.identifier.as_str())
            .style(config::style_header())
            .alignment(Alignment::Center)
            .render(header, buf);

        self.render_content(content, buf);

        self.render_info_popup(area, buf);
        self.render_help_popup(area, buf);
    }

    fn render_content(&mut self, area: Rect, buf: &mut Buffer) {
        let counts = self.counts;
        let min_button_gap = config::BTN_COL_GAP.max(config::COMPACT_BTN_COL_GAP);
        let button_width = area.width / config::BUTTONS_PER_ROW as u16;
        let axes_present = counts.total_axes() > 0 && area.width >= config::AXIS_MIN_WIDTH;
        let touch_present = self.touch.enabled() && area.width >= config::TOUCHPAD_MIN_WIDTH;
        let buttons_present = counts.btn > 0 && button_width > min_button_gap;
        let layout = box_layout(area, axes_present, touch_present, buttons_present);
        let axes_box = layout.axes_box;
        let touch_box = layout.touch_box;
        let buttons_box = layout.buttons_box;

        self.axes_box_present = axes_box.is_some();
        self.buttons_box_present = buttons_box.is_some();
        self.sync_focus();

        let axes_inner = axes_box.map(|box_area| {
            self.render_box(box_area, matches!(self.focus, Focus::Axes), " Axes ", buf)
        });
        let buttons_area = buttons_box.map(|box_area| {
            self.render_box(
                box_area,
                matches!(self.focus, Focus::Buttons),
                " Buttons ",
                buf,
            )
        });
        let touch_area = touch_box.map(|box_area| self.render_touchpad_box(box_area, buf));

        let axes_sections = axes_inner.map(|inner| axes_layout(inner, counts.abs, counts.rel));
        let (abs_area, rel_area) = if let Some(sections) = axes_sections {
            (sections.abs_area, sections.rel_area)
        } else {
            (None, None)
        };

        let abs_visible_capacity = abs_area
            .map(|a| AxisRenderer::capacity_for(a, counts.abs))
            .unwrap_or(0);
        let rel_visible_capacity = rel_area
            .map(|a| AxisRenderer::capacity_for(a, counts.rel))
            .unwrap_or(0);

        let abs_visible = abs_visible_capacity > 0;
        let rel_visible = rel_visible_capacity > 0;

        let button_rows_capacity = if let Some(btn_area) = buttons_area {
            self.buttons_visible_rows(btn_area)
        } else {
            0
        };
        let buttons_visible = button_rows_capacity > 0;

        self.effective_counts = counts.filtered(abs_visible, rel_visible, buttons_visible);

        self.abs_max_start =
            Self::aligned_window_start(self.effective_counts.abs, abs_visible_capacity, 1);
        self.rel_max_start =
            Self::aligned_window_start(self.effective_counts.rel, rel_visible_capacity, 1);
        self.axes_scroll_max = self.abs_max_start + self.rel_max_start;

        if abs_visible_capacity + rel_visible_capacity == 0 {
            self.axis_scroll = 0;
        }
        self.axis_scroll = self.axis_scroll.min(self.axes_scroll_max);

        self.axes_overflow = (abs_visible_capacity + rel_visible_capacity) > 0
            && (self.effective_counts.abs > abs_visible_capacity
                || self.effective_counts.rel > rel_visible_capacity);

        let total_button_rows = self.effective_counts.btn.div_ceil(config::BUTTONS_PER_ROW);
        self.button_row_max_start = if button_rows_capacity == 0 {
            0
        } else {
            total_button_rows.saturating_sub(button_rows_capacity)
        };
        if button_rows_capacity == 0 {
            self.button_row_scroll = 0;
        }
        self.button_row_scroll = self.button_row_scroll.min(self.button_row_max_start);

        self.buttons_overflow =
            button_rows_capacity > 0 && total_button_rows > button_rows_capacity;

        let abs_inputs: InputsVec = self.inputs.iter_absolute().collect();
        let rel_inputs: InputsVec = self.inputs.iter_relative().collect();
        let btn_inputs: InputsVec = self.inputs.iter_buttons().collect();

        let (abs_off, rel_off) = self.axis_offsets();
        if let Some(abs_area) = abs_area {
            AxisRenderer::render_axes_with_scroll(&abs_inputs, abs_area, abs_off, buf);
        }

        if let Some(rel_area) = rel_area {
            AxisRenderer::render_axes_with_scroll(&rel_inputs, rel_area, rel_off, buf);
        }

        if let Some(touch_area) = touch_area {
            let active_points = self.touch.active_points();
            let inactive_points = self.touch.inactive_points();
            TouchRenderer::render(
                touch_area,
                &active_points,
                &inactive_points,
                self.touch.x_range(),
                self.touch.y_range(),
                buf,
            );
        }

        if let Some(btn_area) = buttons_area {
            ButtonGrid::render_with_scroll(&btn_inputs, btn_area, self.button_row_scroll, buf);
        }
    }

    fn render_box(&self, area: Rect, focused: bool, title: &str, buf: &mut Buffer) -> Rect {
        if area.height >= 2 && area.width >= 2 {
            let style = if focused {
                config::style_box_focused()
            } else {
                config::style_box_unfocused()
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_alignment(Alignment::Center)
                .border_style(style);
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        }
    }

    fn render_touchpad_box(&self, area: Rect, buf: &mut Buffer) -> Rect {
        if area.height >= 2 && area.width >= 2 {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Touchpad ")
                .title_alignment(Alignment::Center)
                .border_style(config::style_box_unfocused());
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        }
    }

    fn render_info_popup(&self, area: Rect, buf: &mut Buffer) {
        if !self.info_visible {
            return;
        }

        let popup = Popup {
            title: " Device Info ",
            lines: &self.info_popup.lines,
            min_width: 20,
            min_height: 5,
            max_width: None,
            max_height: None,
            text_style: config::style_label(),
            border_style: config::style_box_focused(),
            text_alignment: Alignment::Left,
            title_alignment: Alignment::Center,
            wrap: Wrap { trim: false },
        };
        render_popup(area, buf, &popup);
    }

    fn render_help_popup(&self, area: Rect, buf: &mut Buffer) {
        if !self.help_visible {
            return;
        }
        let popup = Popup {
            title: " Help ",
            lines: &self.help_popup.lines,
            min_width: HELP_POPUP_MIN_WIDTH,
            min_height: HELP_POPUP_MIN_HEIGHT,
            max_width: Some(HELP_POPUP_MAX_WIDTH),
            max_height: None,
            text_style: config::style_label(),
            border_style: config::style_box_focused(),
            text_alignment: Alignment::Left,
            title_alignment: Alignment::Center,
            wrap: Wrap { trim: false },
        };
        render_popup(area, buf, &popup);
    }

    fn handle_event(&mut self, key_event: KeyEvent) -> Command {
        let code = key_event.code;

        match code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::Quit
            }
            KeyCode::Char('r') => Command::Reset,
            KeyCode::Home | KeyCode::Char('g') => Command::Home,
            KeyCode::End | KeyCode::Char('G') => Command::End,
            KeyCode::Up | KeyCode::Char('k') => Command::Scroll(-1),
            KeyCode::Down | KeyCode::Char('j') => Command::Scroll(1),
            KeyCode::Char('i') => Command::ToggleInfo,
            KeyCode::Char('?') => Command::ToggleHelp,
            KeyCode::Char('J') => Command::FocusNext,
            KeyCode::Char('K') => Command::FocusPrev,
            KeyCode::PageUp => Command::Page(-1),
            KeyCode::PageDown => Command::Page(1),
            _ => Command::None,
        }
    }

    fn axis_offsets(&self) -> (usize, usize) {
        axis_offsets_for(
            self.axis_scroll,
            self.effective_counts.abs,
            self.effective_counts.rel,
            self.abs_max_start,
            self.rel_max_start,
        )
    }

    fn buttons_visible_rows(&self, btn_area: Rect) -> usize {
        let metrics = ButtonGrid::metrics(btn_area);
        if metrics.renderable() {
            metrics.max_rows
        } else {
            0
        }
    }

    fn aligned_window_start(count: usize, capacity: usize, align: usize) -> usize {
        if capacity == 0 || count == 0 {
            return 0;
        }
        let max_start = count.saturating_sub(capacity);
        if align <= 1 {
            max_start
        } else {
            (max_start / align) * align
        }
    }
}

#[cfg(test)]
mod tests {
    use super::axis_offsets_for;

    #[test]
    fn axis_offsets_scroll_rel_after_abs() {
        assert_eq!(axis_offsets_for(6, 10, 10, 6, 6), (6, 0));
        assert_eq!(axis_offsets_for(7, 10, 10, 6, 6), (6, 1));
        assert_eq!(axis_offsets_for(12, 10, 10, 6, 6), (6, 6));
    }

    #[test]
    fn axis_offsets_clamp_in_buttons_region() {
        assert_eq!(axis_offsets_for(25, 10, 10, 6, 6), (6, 6));
    }

    #[test]
    fn axis_offsets_rel_only() {
        assert_eq!(axis_offsets_for(1, 0, 5, 0, 2), (0, 1));
        assert_eq!(axis_offsets_for(4, 0, 5, 0, 2), (0, 2));
    }

    #[test]
    fn axis_offsets_abs_present_no_scroll() {
        assert_eq!(axis_offsets_for(1, 2, 6, 0, 3), (0, 1));
        assert_eq!(axis_offsets_for(3, 2, 6, 0, 3), (0, 3));
    }
}
