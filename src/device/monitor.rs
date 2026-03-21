mod config;
mod controls;
mod layout;
mod math;
mod model;
mod render;
mod theme;
mod touch;
mod ui;

use crossterm::event::{
    Event, EventStream as TermEventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use evdev::AbsoluteAxisCode;
use futures::StreamExt;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Alignment, Rect},
    widgets::{Paragraph, Widget, Wrap},
};
use tokio::select;

use self::{
    controls::Command,
    layout::{axes_layout, box_layout, main_layout, split_buttons_column},
    model::{InputCollection, InputsVec},
    render::{
        axis::AxisRenderer,
        buttons::ButtonGrid,
        hat::{HatRenderer, HatState},
        joystick::{JoystickRenderer, JoystickState},
        touch::TouchRenderer,
    },
    touch::TouchState,
};
use crate::{
    device::{
        popup::{Popup, render_popup},
        selector::DeviceInfo,
        widgets,
    },
    error::{Error, ErrorArea, Result},
};

const HELP_POPUP_MIN_WIDTH: u16 = 30;
const HELP_POPUP_MIN_HEIGHT: u16 = 6;
const HELP_POPUP_MAX_WIDTH: u16 = 80;
const HELP_POPUP_LINES: &[&str] = &[
    "Scroll: Up/Down or j/k, PageUp/PageDown",
    "Jump: Home/End or g/G",
    "Reset: r (relative axes)",
    "Info: i (press i or Esc to close)",
    "Invert Y: y",
    "Focus: Shift+J/Shift+K (when axes and buttons show)",
    "Back: Esc (when no popup is open)",
    "Exit app: Ctrl-C",
    "Help: ? (press ? or Esc to close)",
];

pub(crate) enum MonitorExit {
    BackToSelector,
    ExitApp,
}

pub(super) struct ComponentBootstrap<T> {
    pub(super) value: T,
    pub(super) startup_warnings: Vec<String>,
}

impl<T> ComponentBootstrap<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            startup_warnings: Vec::new(),
        }
    }
}

struct DeviceBootstrap {
    inputs: InputCollection,
    touch: TouchState,
    startup_warnings: Vec<String>,
}

impl DeviceBootstrap {
    fn from_device(device: &evdev::Device) -> Self {
        let inputs = InputCollection::from_device(device);
        let touch = TouchState::from_device(device);
        let mut startup_warnings = inputs.startup_warnings;
        startup_warnings.extend(touch.startup_warnings);

        Self {
            inputs: inputs.value,
            touch: touch.value,
            startup_warnings,
        }
    }
}

pub struct DeviceMonitor {
    device_stream: evdev::EventStream,
    inputs: InputCollection,
    identifier: String,
    counts: Counts,
    // Counts adjusted to what is actually renderable in the current layout
    effective_counts: Counts,
    info_popup: DeviceInfoPopup,
    active_popup: ActivePopup,
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
    joystick_invert_y: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActivePopup {
    None,
    Info,
    Help,
}

struct DeviceInfoPopup {
    lines: Vec<String>,
}

impl DeviceInfoPopup {
    fn new(
        driver_version: (u8, u8, u8),
        input_id: evdev::InputId,
        phys: Option<&str>,
        startup_warnings: &[String],
    ) -> Self {
        let (major, minor, patch) = driver_version;
        let bus = input_id.bus_type().0;
        let vendor = input_id.vendor();
        let product = input_id.product();
        let version = input_id.version();
        let phys = phys.unwrap_or("n/a");
        let mut lines = vec![
            format!("Input driver version: {major}.{minor}.{patch}"),
            format!(
                "Input device ID: bus {bus:#x}, vendor {vendor:#x}, product {product:#x}, version {version:#x}"
            ),
            format!("Input device phys: {phys}"),
        ];
        for warning in startup_warnings {
            lines.push(format!("Startup warning: {warning}"));
        }

        Self { lines }
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

fn command_for(key_event: KeyEvent, popup: ActivePopup) -> Command {
    match popup {
        ActivePopup::Info => match key_event.code {
            KeyCode::Esc | KeyCode::Char('i') => Command::ToggleInfo,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            _ => Command::None,
        },
        ActivePopup::Help => match key_event.code {
            KeyCode::Esc | KeyCode::Char('?') => Command::ToggleHelp,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            _ => Command::None,
        },
        ActivePopup::None => match key_event.code {
            KeyCode::Esc => Command::BackToSelector,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            KeyCode::Char('r') => Command::Reset,
            KeyCode::Home | KeyCode::Char('g') => Command::Home,
            KeyCode::End | KeyCode::Char('G') => Command::End,
            KeyCode::Up | KeyCode::Char('k') => Command::Scroll(-1),
            KeyCode::Down | KeyCode::Char('j') => Command::Scroll(1),
            KeyCode::Char('i') => Command::ToggleInfo,
            KeyCode::Char('y') => Command::ToggleInvertY,
            KeyCode::Char('?') => Command::ToggleHelp,
            KeyCode::Char('J') => Command::FocusNext,
            KeyCode::Char('K') => Command::FocusPrev,
            KeyCode::PageUp => Command::Page(-1),
            KeyCode::PageDown => Command::Page(1),
            _ => Command::None,
        },
    }
}

fn toggled_popup(current: ActivePopup, target: ActivePopup) -> ActivePopup {
    if current == target {
        ActivePopup::None
    } else {
        target
    }
}

fn next_focus(current: Focus, focusable: bool) -> Focus {
    if !focusable {
        return current;
    }

    match current {
        Focus::Axes => Focus::Buttons,
        Focus::Buttons => Focus::Axes,
    }
}

fn synced_focus(current: Focus, axes_box_present: bool, buttons_box_present: bool) -> Focus {
    match (axes_box_present, buttons_box_present) {
        (true, true) => current,
        (true, false) => Focus::Axes,
        (false, true) => Focus::Buttons,
        (false, false) => current,
    }
}

impl DeviceMonitor {
    fn new(DeviceInfo { device, identifier }: DeviceInfo) -> Result<Self> {
        let bootstrap = DeviceBootstrap::from_device(&device);
        let info_popup = DeviceInfoPopup::new(
            device.driver_version(),
            device.input_id(),
            device.physical_path(),
            &bootstrap.startup_warnings,
        );
        let device_stream = device.into_event_stream().map_err(|err| {
            Error::evdev(
                ErrorArea::Monitor,
                format!("open device stream ({identifier})"),
                err,
            )
        })?;
        let counts = Counts {
            abs: bootstrap.inputs.iter_absolute().count(),
            rel: bootstrap.inputs.iter_relative().count(),
            btn: bootstrap.inputs.iter_buttons().count(),
        };
        let focus = if counts.total_axes() > 0 {
            Focus::Axes
        } else {
            Focus::Buttons
        };
        Ok(Self {
            device_stream,
            inputs: bootstrap.inputs,
            identifier,
            effective_counts: counts,
            counts,
            info_popup,
            active_popup: ActivePopup::None,
            touch: bootstrap.touch,
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
            joystick_invert_y: true,
        })
    }

    pub async fn run(
        terminal: &mut DefaultTerminal,
        device_info: DeviceInfo,
    ) -> Result<MonitorExit> {
        let mut monitor = Self::new(device_info)?;
        let mut term_events = TermEventStream::new();

        loop {
            terminal
                .draw(|frame| monitor.render(frame.area(), frame.buffer_mut()))
                .map_err(|err| Error::io(ErrorArea::Monitor, "monitor draw", err))?;

            select! {
                event = term_events.next() => {
                    match event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                            match command_for(key, monitor.active_popup) {
                                Command::BackToSelector => {
                                    return Ok(MonitorExit::BackToSelector);
                                }
                                Command::ExitApp => return Ok(MonitorExit::ExitApp),
                                Command::Reset => monitor.inputs.reset_relative_axes(),
                                Command::Scroll(dir) => monitor.scroll_by(dir),
                                Command::Page(dir) => monitor.scroll_page(dir),
                                Command::Home => monitor.scroll_home(),
                                Command::End => monitor.scroll_end(),
                                Command::FocusNext => monitor.focus_next(),
                                Command::FocusPrev => monitor.focus_prev(),
                                Command::ToggleInfo => monitor.toggle_info(),
                                Command::ToggleHelp => monitor.toggle_help(),
                                Command::ToggleInvertY => monitor.toggle_invert_y(),
                                Command::None => {}
                            }
                        }
                        Some(Ok(_)) => {}
                        Some(Err(err)) => {
                            return Err(Error::io(
                                ErrorArea::Monitor,
                                "terminal event stream",
                                err,
                            ));
                        }
                        None => {
                            return Err(Error::stream_ended(
                                ErrorArea::Monitor,
                                "terminal event stream",
                            ));
                        }
                    }
                }
                event = monitor.device_stream.next_event() => {
                    let event = event.map_err(|err| {
                        Error::evdev(
                            ErrorArea::Monitor,
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
        self.focus = next_focus(self.focus, self.focusable());
    }

    fn focus_prev(&mut self) {
        self.focus_next();
    }

    fn toggle_info(&mut self) {
        self.active_popup = toggled_popup(self.active_popup, ActivePopup::Info);
    }

    fn toggle_help(&mut self) {
        self.active_popup = toggled_popup(self.active_popup, ActivePopup::Help);
    }

    fn focusable(&self) -> bool {
        self.axes_box_present && self.buttons_box_present
    }

    fn sync_focus(&mut self) {
        self.focus = synced_focus(self.focus, self.axes_box_present, self.buttons_box_present);
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

        match self.active_popup {
            ActivePopup::None => {}
            ActivePopup::Info => self.render_info_popup(area, buf),
            ActivePopup::Help => self.render_help_popup(area, buf),
        }
    }

    fn toggle_invert_y(&mut self) {
        self.joystick_invert_y = !self.joystick_invert_y;
    }

    fn render_content(&mut self, area: Rect, buf: &mut Buffer) {
        let counts = self.counts;
        let min_button_gap = config::BTN_COL_GAP.max(config::COMPACT_BTN_COL_GAP);
        let buttons_available = counts.btn > 0;
        let joystick = if self.touch.is_touch_device() {
            JoystickState::default()
        } else {
            JoystickState::from_axes(
                self.inputs
                    .absolute_axis_pair(AbsoluteAxisCode::ABS_X, AbsoluteAxisCode::ABS_Y),
                self.inputs
                    .absolute_axis_pair(AbsoluteAxisCode::ABS_RX, AbsoluteAxisCode::ABS_RY),
            )
        };
        let hat_state = if self.touch.is_touch_device() {
            None
        } else {
            self.inputs
                .absolute_axis_pair(AbsoluteAxisCode::ABS_HAT0X, AbsoluteAxisCode::ABS_HAT0Y)
                .map(|(x, y)| HatState::from_axes(x, y, self.joystick_invert_y))
        };
        let joystick_count = joystick.count();
        let joystick_present = joystick_count > 0;
        let hat_present = hat_state.is_some();

        let axes_available = counts.total_axes() > 0;
        let touch_enabled = self.touch.enabled();

        let mut main_min_width = config::MAIN_COLUMN_MIN_WIDTH;
        if axes_available {
            main_min_width = main_min_width.max(config::AXIS_MIN_WIDTH);
        }
        if touch_enabled {
            main_min_width = main_min_width.max(config::TOUCHPAD_MIN_WIDTH);
        }
        if joystick_present {
            main_min_width = main_min_width.max(config::JOYSTICK_MIN_SIZE);
        }
        if hat_present {
            main_min_width = main_min_width.max(config::HAT_MIN_SIZE);
        }

        let (main_area, buttons_column) = split_buttons_column(
            area,
            buttons_available,
            main_min_width,
            config::BUTTONS_COLUMN_MIN_WIDTH,
            min_button_gap,
        );

        let axes_present = axes_available && main_area.width >= config::AXIS_MIN_WIDTH;
        let touch_present = touch_enabled && main_area.width >= config::TOUCHPAD_MIN_WIDTH;
        let button_width = main_area.width / config::BUTTONS_PER_ROW as u16;
        let buttons_present = buttons_available && button_width > min_button_gap;

        let (layout, buttons_box) = if let Some(buttons_area) = buttons_column {
            let layout = box_layout(
                main_area,
                joystick_present,
                joystick_count,
                hat_present,
                touch_present,
                axes_present,
                false,
            );
            (layout, Some(buttons_area))
        } else {
            let layout = box_layout(
                main_area,
                joystick_present,
                joystick_count,
                hat_present,
                touch_present,
                axes_present,
                buttons_present,
            );
            let buttons_box = layout.buttons_box;
            (layout, buttons_box)
        };
        let joystick_box = layout.joystick_box;
        let hat_box = layout.hat_box;
        let axes_box = layout.axes_box;
        let touch_box = layout.touch_box;

        self.axes_box_present = axes_box.is_some();
        self.buttons_box_present = buttons_box.is_some();
        self.sync_focus();

        let axes_inner = axes_box.map(|box_area| {
            self.render_box(box_area, matches!(self.focus, Focus::Axes), " Axes ", buf)
        });
        let joystick_area =
            joystick_box.map(|box_area| self.render_joystick_box(box_area, joystick_count, buf));
        let hat_area = hat_box.map(|box_area| self.render_hat_box(box_area, buf));
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

        if let (Some(touch_area), Some((x_range, y_range))) = (touch_area, self.touch.ranges()) {
            let active_points = self.touch.active_points();
            let inactive_points = self.touch.inactive_points();
            TouchRenderer::render(
                touch_area,
                &active_points,
                &inactive_points,
                x_range,
                y_range,
                buf,
            );
        }

        if let Some(joystick_area) = joystick_area {
            JoystickRenderer::render(joystick_area, &joystick, self.joystick_invert_y, buf);
        }

        if let (Some(hat_area), Some(hat_state)) = (hat_area, hat_state) {
            HatRenderer::render(hat_area, hat_state, buf);
        }

        if let Some(btn_area) = buttons_area {
            ButtonGrid::render_with_scroll(&btn_inputs, btn_area, self.button_row_scroll, buf);
        }
    }

    fn render_box(&self, area: Rect, focused: bool, title: &str, buf: &mut Buffer) -> Rect {
        let style = if focused {
            config::style_box_focused()
        } else {
            config::style_box_unfocused()
        };
        widgets::render_bordered_titled_box(area, title, style, Alignment::Left, buf)
    }

    fn render_touchpad_box(&self, area: Rect, buf: &mut Buffer) -> Rect {
        widgets::render_bordered_titled_box(
            area,
            " Touchpad ",
            config::style_box_unfocused(),
            Alignment::Left,
            buf,
        )
    }

    fn render_joystick_box(&self, area: Rect, count: usize, buf: &mut Buffer) -> Rect {
        let title = if count > 1 {
            " Joysticks "
        } else {
            " Joystick "
        };
        widgets::render_bordered_titled_box(
            area,
            title,
            config::style_box_unfocused(),
            Alignment::Left,
            buf,
        )
    }

    fn render_hat_box(&self, area: Rect, buf: &mut Buffer) -> Rect {
        widgets::render_bordered_titled_box(
            area,
            " D-pad ",
            config::style_box_unfocused(),
            Alignment::Left,
            buf,
        )
    }

    fn render_info_popup(&self, area: Rect, buf: &mut Buffer) {
        let popup = Popup::new(" Device Info ")
            .min_size(20, 5)
            .text_style(config::style_label())
            .border_style(config::style_box_focused())
            .text_alignment(Alignment::Left)
            .title_alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        render_popup(area, buf, &popup, &self.info_popup.lines);
    }

    fn render_help_popup(&self, area: Rect, buf: &mut Buffer) {
        let popup = Popup::new(" Help ")
            .min_size(HELP_POPUP_MIN_WIDTH, HELP_POPUP_MIN_HEIGHT)
            .max_width(HELP_POPUP_MAX_WIDTH)
            .text_style(config::style_label())
            .border_style(config::style_box_focused())
            .text_alignment(Alignment::Left)
            .title_alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        render_popup(area, buf, &popup, HELP_POPUP_LINES);
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
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{
        ActivePopup, DeviceMonitor, Focus, axis_offsets_for, command_for, next_focus, synced_focus,
        toggled_popup,
    };
    use crate::device::monitor::controls::Command;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

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

    #[test]
    fn command_for_ctrl_c_exits_from_any_popup_state() {
        for popup in [ActivePopup::None, ActivePopup::Info, ActivePopup::Help] {
            assert_eq!(command_for(ctrl_char('c'), popup), Command::ExitApp);
        }
    }

    #[test]
    fn command_for_escape_backs_out_only_without_popup() {
        assert_eq!(
            command_for(key(KeyCode::Esc), ActivePopup::None),
            Command::BackToSelector
        );
        assert_eq!(
            command_for(key(KeyCode::Esc), ActivePopup::Info),
            Command::ToggleInfo
        );
        assert_eq!(
            command_for(key(KeyCode::Esc), ActivePopup::Help),
            Command::ToggleHelp
        );
    }

    #[test]
    fn toggled_popup_switches_between_help_and_info() {
        assert_eq!(
            toggled_popup(ActivePopup::None, ActivePopup::Info),
            ActivePopup::Info
        );
        assert_eq!(
            toggled_popup(ActivePopup::Info, ActivePopup::Info),
            ActivePopup::None
        );
        assert_eq!(
            toggled_popup(ActivePopup::Help, ActivePopup::Info),
            ActivePopup::Info
        );
    }

    #[test]
    fn next_focus_cycles_only_when_both_sections_are_focusable() {
        assert_eq!(next_focus(Focus::Axes, true), Focus::Buttons);
        assert_eq!(next_focus(Focus::Buttons, true), Focus::Axes);
        assert_eq!(next_focus(Focus::Axes, false), Focus::Axes);
    }

    #[test]
    fn synced_focus_forces_the_remaining_visible_section() {
        assert_eq!(synced_focus(Focus::Axes, true, true), Focus::Axes);
        assert_eq!(synced_focus(Focus::Axes, true, false), Focus::Axes);
        assert_eq!(synced_focus(Focus::Axes, false, true), Focus::Buttons);
        assert_eq!(synced_focus(Focus::Buttons, false, false), Focus::Buttons);
    }

    #[test]
    fn aligned_window_start_respects_alignment_step() {
        assert_eq!(DeviceMonitor::aligned_window_start(10, 3, 1), 7);
        assert_eq!(DeviceMonitor::aligned_window_start(10, 3, 2), 6);
        assert_eq!(DeviceMonitor::aligned_window_start(2, 5, 3), 0);
    }
}
