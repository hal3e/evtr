mod config;
mod controls;
mod layout;
mod math;
mod model;
mod render;
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
    widgets::{Paragraph, Widget},
};
use tokio::select;

use crate::error::{Error, Result};

use self::{
    controls::Command,
    layout::{SectionSizer, main_layout},
    model::{InputCollection, InputsVec},
    render::{axis::AxisRenderer, buttons::ButtonGrid},
};
use crate::device::DeviceInfo;

pub struct DeviceMonitor {
    device_stream: evdev::EventStream,
    inputs: InputCollection,
    scroll: ScrollState,
    last_content_area_height: u16,
    identifier: String,
    counts: Counts,
    // Counts adjusted to what is actually renderable in the current layout
    effective_counts: Counts,
    // Max scroll steps across axes (abs then rel).
    axes_scroll_max: usize,
    abs_max_start: usize,
    rel_max_start: usize,
    // Max starting offset (global) for buttons page start, aligned to row starts
    buttons_max_start: usize,
    last_overflow: bool,
    scroll_seeded: bool,
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
    fn btn_rows(&self) -> usize {
        self.btn.div_ceil(config::BUTTONS_PER_ROW)
    }
    fn max_offset(&self) -> usize {
        let total_axes = self.total_axes();
        if self.btn == 0 {
            total_axes.saturating_sub(1)
        } else {
            let last_row_start = self
                .btn_rows()
                .saturating_sub(1)
                .saturating_mul(config::BUTTONS_PER_ROW);
            total_axes.saturating_add(last_row_start)
        }
    }

    fn filtered(&self, abs_visible: bool, rel_visible: bool, buttons_visible: bool) -> Self {
        Self {
            abs: if abs_visible { self.abs } else { 0 },
            rel: if rel_visible { self.rel } else { 0 },
            btn: if buttons_visible { self.btn } else { 0 },
        }
    }
}

#[derive(Debug, Default)]
struct ScrollState {
    offset: usize,
}

impl ScrollState {
    fn new() -> Self {
        Self { offset: 0 }
    }

    fn scroll_page(
        &mut self,
        counts: &Counts,
        axes_scroll_max: usize,
        buttons_max_start: usize,
        dir: i32,
    ) {
        for _ in 0..config::PAGE_SCROLL_STEPS {
            self.scroll_step(counts, axes_scroll_max, buttons_max_start, dir);
        }
    }

    fn scroll_up(&mut self, counts: &Counts, axes_scroll_max: usize, buttons_max_start: usize) {
        self.scroll_step(counts, axes_scroll_max, buttons_max_start, -1);
    }

    fn scroll_down(&mut self, counts: &Counts, axes_scroll_max: usize, buttons_max_start: usize) {
        self.scroll_step(counts, axes_scroll_max, buttons_max_start, 1);
    }

    fn scroll_step(
        &mut self,
        counts: &Counts,
        axes_scroll_max: usize,
        buttons_max_start: usize,
        direction: i32,
    ) {
        let total_axes = counts.total_axes();
        if direction < 0 {
            // Up
            if self.offset > total_axes {
                // Within button rows: go to previous row start
                let button_offset = self.offset - total_axes;
                let current_button_row = button_offset / config::BUTTONS_PER_ROW;
                if current_button_row > 0 {
                    self.offset = total_axes + ((current_button_row - 1) * config::BUTTONS_PER_ROW);
                } else {
                    // From first button row back into axes: snap to end-of-axes window
                    self.offset = axes_scroll_max;
                }
            } else if self.offset == total_axes && total_axes > 0 {
                // Exactly at boundary, snap to end-of-axes window
                self.offset = axes_scroll_max;
            } else if self.offset > 0 {
                // Within axes, step up by one
                self.offset -= 1;
            }
        } else if direction > 0 {
            // Down
            if self.offset < total_axes {
                // Within axes: cap at last visible start; then jump to buttons
                if self.offset < axes_scroll_max {
                    self.offset += 1;
                } else if counts.btn > 0 {
                    // Jump to next button row when buttons already render.
                    let next_row = total_axes + config::BUTTONS_PER_ROW;
                    self.offset = next_row.min(buttons_max_start);
                }
            } else {
                // Within buttons: advance by full rows
                let button_offset = self.offset - total_axes;
                let current_button_row = button_offset / config::BUTTONS_PER_ROW;
                let total_button_rows = counts.btn_rows();
                let next = total_axes + ((current_button_row + 1) * config::BUTTONS_PER_ROW);
                self.offset = self.offset.min(buttons_max_start);
                if current_button_row + 1 < total_button_rows {
                    // Move, but never exceed the last full-page start
                    self.offset = next.min(buttons_max_start);
                }
            }
        }
    }

    fn button_row_offset(&self, total_axes: usize) -> usize {
        let button_scroll_offset = self.offset.saturating_sub(total_axes);
        button_scroll_offset / config::BUTTONS_PER_ROW
    }

    fn align_for_buttons(&mut self, total_axes: usize) {
        if self.offset >= total_axes {
            let button_index = self.offset - total_axes;
            let row_aligned = (button_index / config::BUTTONS_PER_ROW) * config::BUTTONS_PER_ROW;
            self.offset = total_axes + row_aligned;
        }
    }

    fn clamp_and_align(
        &mut self,
        counts: &Counts,
        axes_scroll_max: usize,
        buttons_max_start: usize,
    ) {
        let total_axes = counts.total_axes();
        let max_offset = counts.max_offset();
        self.offset = self.offset.min(max_offset);
        if self.offset < total_axes {
            // Clamp axes scroll to last fully-visible page.
            self.offset = self.offset.min(axes_scroll_max);
        } else {
            // Align button scroll to the start of a row.
            self.align_for_buttons(total_axes);
            // Do not allow starts beyond the last full-page start.
            self.offset = self.offset.min(buttons_max_start);
        }
    }
}

fn axis_offsets_for(
    scroll_offset: usize,
    total_axes: usize,
    abs_count: usize,
    rel_count: usize,
    abs_max_start: usize,
    rel_max_start: usize,
) -> (usize, usize) {
    let axes_scroll_max = abs_max_start + rel_max_start;
    let axis_scroll = if scroll_offset >= total_axes {
        axes_scroll_max
    } else {
        scroll_offset.min(axes_scroll_max)
    };

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
        let device_stream = device
            .into_event_stream()
            .map_err(|err| Error::evdev(format!("open device stream ({identifier})"), err))?;
        let counts = Counts {
            abs: inputs.iter_absolute().count(),
            rel: inputs.iter_relative().count(),
            btn: inputs.iter_buttons().count(),
        };
        Ok(Self {
            device_stream,
            inputs,
            scroll: ScrollState::new(),
            last_content_area_height: 40,
            identifier,
            effective_counts: counts,
            counts,
            axes_scroll_max: 0,
            abs_max_start: 0,
            rel_max_start: 0,
            buttons_max_start: 0,
            last_overflow: false,
            scroll_seeded: false,
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
                            match monitor.handle_event(key) {
                                Command::Quit => return Ok(()),
                                Command::Reset => monitor.inputs.reset_relative_axes(),
                                Command::Scroll(dir) => monitor.scroll_by(dir),
                                Command::Page(dir) => {
                                    let counts = monitor.effective_counts;
                                    let axes_max = monitor.axes_scroll_max;
                                    let buttons_max = monitor.buttons_max_start;
                                    monitor
                                        .scroll
                                        .scroll_page(&counts, axes_max, buttons_max, dir);
                                }
                                Command::Home => monitor.scroll.offset = 0,
                                Command::End => monitor.scroll_to_end(),
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
                }
            }
        }
    }

    fn scroll_by(&mut self, direction: i32) {
        if direction == 0 || !self.has_overflow() {
            return;
        }
        let counts = self.effective_counts;
        let axes_max = self.axes_scroll_max;
        let buttons_max = self.buttons_max_start;
        if direction < 0 {
            self.scroll.scroll_up(&counts, axes_max, buttons_max);
        } else {
            self.scroll.scroll_down(&counts, axes_max, buttons_max);
        }
    }

    fn scroll_to_end(&mut self) {
        if !self.has_overflow() {
            self.scroll.offset = 0;
            return;
        }
        if self.effective_counts.btn == 0 {
            self.scroll.offset = self.axes_scroll_max;
        } else {
            self.scroll.offset = self.effective_counts.max_offset();
            self.scroll
                .align_for_buttons(self.effective_counts.total_axes());
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let [header, content, footer] = main_layout(area);

        Paragraph::new(self.identifier.as_str())
            .style(config::style_header())
            .alignment(Alignment::Center)
            .render(header, buf);

        self.last_content_area_height = content.height;
        let overflow = self.render_content(content, buf);

        let footer_text = self.build_footer_text(&self.effective_counts, overflow);

        Paragraph::new(footer_text)
            .style(config::style_header())
            .alignment(Alignment::Center)
            .render(footer, buf);
    }

    fn render_content(&mut self, area: Rect, buf: &mut Buffer) -> bool {
        let counts = &self.counts;
        let button_width = area.width / config::BUTTONS_PER_ROW as u16;
        let buttons_width_ok = button_width > config::BTN_COL_GAP;
        let btn_rows = if buttons_width_ok {
            counts.btn_rows()
        } else {
            0
        };
        // Derive what is actually renderable for axes in this layout pass
        let sizer = SectionSizer::new(area, btn_rows, counts.abs, counts.rel);

        let abs_visible = Self::axes_renderable(sizer.abs_area, counts.abs);
        let rel_visible = Self::axes_renderable(sizer.rel_area, counts.rel);
        let button_rows_capacity = if counts.btn == 0 {
            0
        } else {
            sizer
                .btn_area
                .map(|btn_area| self.buttons_visible_rows(btn_area))
                .unwrap_or(0)
        };
        let buttons_visible = button_rows_capacity > 0;

        self.effective_counts = counts.filtered(abs_visible, rel_visible, buttons_visible);

        let total_axes = self.effective_counts.total_axes();

        let abs_visible_capacity = sizer
            .abs_area
            .map(|a| AxisRenderer::capacity_for(a, self.effective_counts.abs))
            .unwrap_or(0);
        let rel_visible_capacity = sizer
            .rel_area
            .map(|a| AxisRenderer::capacity_for(a, self.effective_counts.rel))
            .unwrap_or(0);

        self.abs_max_start =
            Self::aligned_window_start(self.effective_counts.abs, abs_visible_capacity, 1);
        self.rel_max_start =
            Self::aligned_window_start(self.effective_counts.rel, rel_visible_capacity, 1);
        self.axes_scroll_max = self.abs_max_start + self.rel_max_start;

        // Compute buttons window capacity and last valid buttons start, row-aligned.
        self.buttons_max_start = if button_rows_capacity == 0 {
            total_axes
        } else {
            let total_button_rows = self.effective_counts.btn.div_ceil(config::BUTTONS_PER_ROW);
            let max_row_start = total_button_rows.saturating_sub(button_rows_capacity);
            total_axes + (max_row_start * config::BUTTONS_PER_ROW)
        };

        // If everything fits, anchor to top; otherwise clamp within range and
        // align to button-row starts when in button region.
        let has_overflow = self.overflow_from_capacity(
            abs_visible_capacity,
            rel_visible_capacity,
            button_rows_capacity,
        );
        self.last_overflow = has_overflow;
        if !has_overflow {
            self.scroll.offset = 0;
            self.scroll_seeded = false;
        } else {
            if !self.scroll_seeded
                && self.axes_scroll_max == 0
                && button_rows_capacity * config::BUTTONS_PER_ROW < self.effective_counts.btn
            {
                self.scroll.offset = total_axes;
                self.scroll_seeded = true;
            }
            self.scroll.clamp_and_align(
                &self.effective_counts,
                self.axes_scroll_max,
                self.buttons_max_start,
            );
        }

        let abs_inputs: InputsVec = self.inputs.iter_absolute().collect();
        let rel_inputs: InputsVec = self.inputs.iter_relative().collect();
        let btn_inputs: InputsVec = self.inputs.iter_buttons().collect();

        let (abs_off, rel_off) = self.axis_offsets();
        if let Some(abs_area) = sizer.abs_area {
            AxisRenderer::render_axes_with_scroll(&abs_inputs, abs_area, abs_off, buf);
        }

        if let Some(rel_area) = sizer.rel_area {
            AxisRenderer::render_axes_with_scroll(&rel_inputs, rel_area, rel_off, buf);
        }

        if let Some(btn_area) = sizer.btn_area {
            let row_offset = self.scroll.button_row_offset(total_axes);
            ButtonGrid::render_with_scroll(&btn_inputs, btn_area, row_offset, buf);
        }

        has_overflow
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
            KeyCode::PageUp => Command::Page(-1),
            KeyCode::PageDown => Command::Page(1),
            _ => Command::None,
        }
    }

    fn has_overflow(&self) -> bool {
        self.last_overflow
    }

    fn axis_offsets(&self) -> (usize, usize) {
        axis_offsets_for(
            self.scroll.offset,
            self.effective_counts.total_axes(),
            self.effective_counts.abs,
            self.effective_counts.rel,
            self.abs_max_start,
            self.rel_max_start,
        )
    }

    fn build_footer_text(&self, counts: &Counts, overflow: bool) -> String {
        let has_relative = counts.rel > 0;
        if overflow {
            let total_items = counts.abs + counts.rel + counts.btn;
            let prefix = Self::footer_prefix(has_relative, true);
            format!(
                "{} Items: {} | Offset: {}",
                prefix, total_items, self.scroll.offset
            )
        } else {
            Self::footer_prefix(has_relative, false).to_string()
        }
    }

    fn overflow_from_capacity(
        &self,
        abs_capacity: usize,
        rel_capacity: usize,
        button_rows_capacity: usize,
    ) -> bool {
        abs_capacity < self.effective_counts.abs
            || rel_capacity < self.effective_counts.rel
            || button_rows_capacity * config::BUTTONS_PER_ROW < self.effective_counts.btn
    }

    fn footer_prefix(has_relative: bool, overflow: bool) -> &'static str {
        match (overflow, has_relative) {
            (true, true) => {
                "Ctrl-C: back | 'r': reset | ↑/↓ or j/k: scroll | PgUp/PgDn: fast | Home/End or g/G: jump |"
            }
            (true, false) => {
                "Ctrl-C: back | ↑/↓ or j/k: scroll | PgUp/PgDn: fast | Home/End or g/G: jump |"
            }
            (false, true) => "Ctrl-C: back | 'r': reset relative axes",
            (false, false) => "Ctrl-C: back",
        }
    }

    fn axes_renderable(area: Option<Rect>, count: usize) -> bool {
        if count == 0 {
            return false;
        }
        if let Some(a) = area {
            a.height >= 1 && a.width >= config::AXIS_MIN_WIDTH
        } else {
            false
        }
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
    use crate::device::monitor::{Counts, ScrollState};

    use super::axis_offsets_for;

    #[test]
    fn axis_offsets_scroll_rel_after_abs() {
        assert_eq!(axis_offsets_for(6, 20, 10, 10, 6, 6), (6, 0));
        assert_eq!(axis_offsets_for(7, 20, 10, 10, 6, 6), (6, 1));
        assert_eq!(axis_offsets_for(12, 20, 10, 10, 6, 6), (6, 6));
    }

    #[test]
    fn axis_offsets_clamp_in_buttons_region() {
        assert_eq!(axis_offsets_for(25, 20, 10, 10, 6, 6), (6, 6));
    }

    #[test]
    fn axis_offsets_rel_only() {
        assert_eq!(axis_offsets_for(1, 5, 0, 5, 0, 2), (0, 1));
        assert_eq!(axis_offsets_for(4, 5, 0, 5, 0, 2), (0, 2));
    }

    #[test]
    fn axis_offsets_abs_present_no_scroll() {
        assert_eq!(axis_offsets_for(1, 8, 2, 6, 0, 3), (0, 1));
        assert_eq!(axis_offsets_for(3, 8, 2, 6, 0, 3), (0, 3));
    }

    #[test]
    fn scroll_step_jumps_to_next_button_row() {
        let counts = Counts {
            abs: 4,
            rel: 4,
            btn: 12,
        };
        let mut scroll = ScrollState { offset: 6 };

        scroll.scroll_step(&counts, 6, 14, 1);

        assert_eq!(scroll.offset, 14);
    }
}
