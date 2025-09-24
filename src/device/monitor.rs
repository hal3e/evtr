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
use tokio::select;

use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Alignment, Rect},
    widgets::{Paragraph, Widget},
};

use self::{
    controls::Command,
    layout::{SectionSizer, main_layout},
    model::{InputCollection, InputSlice, InputsVec},
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
    // Max starting index for axes page (avoid dead-range overshoot)
    axes_max_start: usize,
    // Max starting offset (global) for buttons page start, aligned to row starts
    buttons_max_start: usize,
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
        axes_max_start: usize,
        buttons_max_start: usize,
        dir: i32,
    ) {
        for _ in 0..config::PAGE_SCROLL_STEPS {
            self.scroll_step(counts, axes_max_start, buttons_max_start, dir);
        }
    }

    fn scroll_up(&mut self, counts: &Counts, axes_max_start: usize, buttons_max_start: usize) {
        self.scroll_step(counts, axes_max_start, buttons_max_start, -1);
    }

    fn scroll_down(
        &mut self,
        counts: &Counts,
        axes_max_start: usize,
        buttons_max_start: usize,
    ) {
        self.scroll_step(counts, axes_max_start, buttons_max_start, 1);
    }

    fn scroll_step(
        &mut self,
        counts: &Counts,
        axes_max_start: usize,
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
                    self.offset = axes_max_start;
                }
            } else if self.offset == total_axes && total_axes > 0 {
                // Exactly at boundary, snap to end-of-axes window
                self.offset = axes_max_start;
            } else if self.offset > 0 {
                // Within axes, step up by one
                self.offset -= 1;
            }
        } else if direction > 0 {
            // Down
            if self.offset < total_axes {
                // Within axes: cap at last visible start; then jump to buttons
                if self.offset < axes_max_start {
                    self.offset += 1;
                } else if counts.btn > 0 {
                    // Jump to first button row
                    self.offset = total_axes;
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

    fn axis_offsets(&self, abs_count: usize) -> (usize, usize) {
        (self.offset, self.offset.saturating_sub(abs_count))
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
        axes_max_start: usize,
        buttons_max_start: usize,
    ) {
        let total_axes = counts.total_axes();
        let max_offset = counts.max_offset();
        self.offset = self.offset.min(max_offset);
        if self.offset < total_axes {
            // Clamp axes scroll to last fully-visible page.
            self.offset = self.offset.min(axes_max_start);
        } else {
            // Align button scroll to the start of a row.
            self.align_for_buttons(total_axes);
            // Do not allow starts beyond the last full-page start.
            self.offset = self.offset.min(buttons_max_start);
        }
    }
}

impl DeviceMonitor {
    fn new(
        DeviceInfo { device, identifier }: DeviceInfo,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let inputs = InputCollection::from_device(&device);
        let device_stream = device.into_event_stream()?;
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
            axes_max_start: 0,
            buttons_max_start: 0,
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
                            match monitor.handle_event(key) {
                                Command::Quit => return Ok(true),
                                Command::Reset => monitor.inputs.reset_relative_axes(),
                                Command::Scroll(dir) => { if dir < 0 { monitor.scroll_up(); } else { monitor.scroll_down(); } }
                                Command::Page(dir) => monitor.scroll_page(dir),
                                Command::Home => monitor.scroll.offset = 0,
                                Command::End => monitor.scroll_to_end(),
                                Command::None => {}
                            }
                        }
                    } else if let Some(Err(e)) = event { return Err(Box::new(e)); }
                }
                event = monitor.device_stream.next_event() => { monitor.inputs.handle_event(&event?); }
            }
        }
    }

    fn scroll_page(&mut self, dir: i32) {
        self.scroll.scroll_page(
            &self.effective_counts,
            self.axes_max_start,
            self.buttons_max_start,
            dir,
        );
    }

    fn scroll_to_end(&mut self) {
        if !self.has_overflow() {
            self.scroll.offset = 0;
            return;
        }
        self.scroll.offset = self.effective_counts.max_offset();
        self.scroll
            .align_for_buttons(self.effective_counts.total_axes());
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let [header, content, footer] = main_layout(area);

        Paragraph::new(self.identifier.as_str())
            .style(config::style_header())
            .alignment(Alignment::Center)
            .render(header, buf);

        self.last_content_area_height = content.height;
        self.render_content(content, buf);

        let overflow = self.overflow_from_counts(&self.effective_counts);
        let footer_text = self.build_footer_text(&self.effective_counts, overflow);

        Paragraph::new(footer_text)
            .style(config::style_header())
            .alignment(Alignment::Center)
            .render(footer, buf);
    }

    fn render_content(&mut self, area: Rect, buf: &mut Buffer) {
        let counts = &self.counts;
        // Derive what is actually renderable for axes in this layout pass
        let sizer = SectionSizer::new(area, counts.btn_rows(), counts.abs, counts.rel);

        let abs_visible = Self::axes_renderable(sizer.abs_area, counts.abs);
        let rel_visible = Self::axes_renderable(sizer.rel_area, counts.rel);

        let effective_abs = if abs_visible { counts.abs } else { 0 };
        let effective_rel = if rel_visible { counts.rel } else { 0 };
        // If the button area is not present in the current layout, do not
        // include buttons in effective scrollable content.
        let effective_btn = if sizer.btn_area.is_some() { counts.btn } else { 0 };
        self.effective_counts = Counts {
            abs: effective_abs,
            rel: effective_rel,
            btn: effective_btn,
        };

        // Compute axes window capacity and last valid axes start index to
        // avoid dead-range overshoot at the end of the axes sections.
        let axes_visible_capacity = self.axes_visible_capacity(&sizer);
        let total_axes = self.effective_counts.total_axes();
        self.axes_max_start = if axes_visible_capacity > 0 {
            total_axes.saturating_sub(axes_visible_capacity)
        } else {
            0
        };

        // Compute buttons window capacity and last valid buttons start, row-aligned.
        self.buttons_max_start = if let Some(btn_area) = sizer.btn_area {
            let cap = self.buttons_visible_capacity(btn_area);
            let btns = self.effective_counts.btn;
            let start_index = btns.saturating_sub(cap);
            let row_aligned = (start_index / config::BUTTONS_PER_ROW) * config::BUTTONS_PER_ROW;
            total_axes + row_aligned
        } else {
            total_axes
        };

        // If everything fits, anchor to top; otherwise clamp within range and
        // align to button-row starts when in button region.
        if !self.overflow_from_counts(&self.effective_counts) {
            self.scroll.offset = 0;
        } else {
            self.scroll.clamp_and_align(
                &self.effective_counts,
                self.axes_max_start,
                self.buttons_max_start,
            );
        }

        let total_axes = self.effective_counts.total_axes();
        let abs_inputs: InputsVec = self.inputs.iter_absolute().collect();
        let rel_inputs: InputsVec = self.inputs.iter_relative().collect();
        let btn_inputs: InputsVec = self.inputs.iter_buttons().collect();

        if let Some(abs_area) = sizer.abs_area {
            let (abs_off, _) = self.scroll.axis_offsets(self.effective_counts.abs);
            AxisRenderer::render_axes_with_scroll(&abs_inputs, abs_area, abs_off, buf);
        }

        if let Some(rel_area) = sizer.rel_area {
            let (_, rel_off) = self.scroll.axis_offsets(self.effective_counts.abs);
            AxisRenderer::render_axes_with_scroll(&rel_inputs, rel_area, rel_off, buf);
        }

        if let Some(btn_area) = sizer.btn_area {
            self.render_buttons(btn_area, &btn_inputs, total_axes, buf);
        }
    }

    fn scroll_up(&mut self) {
        if !self.has_overflow() {
            return;
        }
        self.scroll
            .scroll_up(&self.effective_counts, self.axes_max_start, self.buttons_max_start);
    }

    fn scroll_down(&mut self) {
        if !self.has_overflow() {
            return;
        }
        self.scroll.scroll_down(
            &self.effective_counts,
            self.axes_max_start,
            self.buttons_max_start,
        );
    }

    fn handle_event(&mut self, key_event: KeyEvent) -> Command {
        let code = key_event.code;

        match code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::Quit
            }
            KeyCode::Char('r') => Command::Reset,
            KeyCode::Up | KeyCode::Char('k') => Command::Scroll(-1),
            KeyCode::Down | KeyCode::Char('j') => Command::Scroll(1),
            KeyCode::PageUp => Command::Page(-1),
            KeyCode::PageDown => Command::Page(1),
            KeyCode::Home => Command::Home,
            KeyCode::End => Command::End,
            _ => Command::None,
        }
    }

    fn has_overflow(&self) -> bool {
        self.overflow_from_counts(&self.effective_counts)
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

    fn button_row_offset(&self, total_axes: usize) -> usize {
        self.scroll.button_row_offset(total_axes)
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
        let (min_axes_height, btn_height) =
            layout::section_min_heights(counts.btn_rows(), counts.abs, counts.rel);
        let total_min_height = min_axes_height + btn_height;
        total_min_height > self.last_content_area_height
    }

    fn footer_prefix(has_relative: bool, overflow: bool) -> &'static str {
        match (overflow, has_relative) {
            (true, true) => "Ctrl-C: back | 'r': reset | ↑/↓ or j/k: scroll | PgUp/PgDn: fast | Home/End: jump |",
            (true, false) => "Ctrl-C: back | ↑/↓ or j/k: scroll | PgUp/PgDn: fast | Home/End: jump |",
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

    fn axes_visible_capacity(&self, sizer: &SectionSizer) -> usize {
        let abs_cap = Self::max_visible_in_area(sizer.abs_area, self.effective_counts.abs);
        let rel_cap = Self::max_visible_in_area(sizer.rel_area, self.effective_counts.rel);
        abs_cap + rel_cap
    }

    fn max_visible_in_area(area: Option<Rect>, count: usize) -> usize {
        if count == 0 {
            return 0;
        }
        if let Some(a) = area {
            if a.height == 0 || a.width < config::AXIS_MIN_WIDTH {
                return 0;
            }
            let bar_height = Self::choose_bar_height(a.height, count);
            let item_height = bar_height + config::AXIS_GAP;
            let capacity = (a.height / item_height) as usize;
            capacity.min(count)
        } else {
            0
        }
    }

    fn choose_bar_height(available_height: u16, num_items: usize) -> u16 {
        if num_items == 0 {
            return 1;
        }
        for &height in &config::BAR_HEIGHTS {
            let total_needed = (height + config::AXIS_GAP) * num_items as u16;
            if total_needed <= available_height {
                return height;
            }
        }
        1
    }

    fn buttons_visible_capacity(&self, btn_area: Rect) -> usize {
        if self.effective_counts.btn == 0 {
            return 0;
        }
        if btn_area.height <= config::BTN_SECTION_VERT_PADDING {
            return 0;
        }
        let usable_h = btn_area.height.saturating_sub(config::BTN_SECTION_VERT_PADDING);
        let max_rows = (usable_h / config::BUTTON_HEIGHT) as usize;
        (max_rows * config::BUTTONS_PER_ROW).min(self.effective_counts.btn)
    }
}
