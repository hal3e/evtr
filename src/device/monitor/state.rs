use crate::device::monitor::plan::{Counts, RenderPlan};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Focus {
    Axes,
    Buttons,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActivePopup {
    None,
    Info,
    Help,
}

pub(crate) struct MonitorState {
    counts: Counts,
    info_lines: Vec<String>,
    active_popup: ActivePopup,
    focus: Focus,
    axis_scroll: usize,
    button_row_scroll: usize,
    joystick_invert_y: bool,
}

impl MonitorState {
    pub(crate) fn new(counts: Counts, info_lines: Vec<String>) -> Self {
        let focus = if counts.total_axes() > 0 {
            Focus::Axes
        } else {
            Focus::Buttons
        };

        Self {
            counts,
            info_lines,
            active_popup: ActivePopup::None,
            focus,
            axis_scroll: 0,
            button_row_scroll: 0,
            joystick_invert_y: true,
        }
    }

    pub(crate) fn counts(&self) -> Counts {
        self.counts
    }

    pub(crate) fn info_lines(&self) -> &[String] {
        &self.info_lines
    }

    pub(crate) fn active_popup(&self) -> ActivePopup {
        self.active_popup
    }

    pub(crate) fn focus(&self) -> Focus {
        self.focus
    }

    pub(crate) fn axis_scroll(&self) -> usize {
        self.axis_scroll
    }

    pub(crate) fn button_row_scroll(&self) -> usize {
        self.button_row_scroll
    }

    pub(crate) fn joystick_invert_y(&self) -> bool {
        self.joystick_invert_y
    }

    pub(crate) fn sync_from_plan(&mut self, plan: &RenderPlan) {
        self.focus = plan.focus;
        self.axis_scroll = plan.scroll.axis;
        self.button_row_scroll = plan.scroll.button_row;
    }

    pub(crate) fn scroll_by(&mut self, direction: i32, plan: &RenderPlan) {
        if direction == 0 {
            return;
        }

        match plan.focus {
            Focus::Axes => step_scroll(
                &mut self.axis_scroll,
                direction,
                plan.scroll_bounds.axes_overflow,
                plan.scroll_bounds.axes_max,
            ),
            Focus::Buttons => step_scroll(
                &mut self.button_row_scroll,
                direction,
                plan.scroll_bounds.buttons_overflow,
                plan.scroll_bounds.button_row_max_start,
            ),
        }
    }

    pub(crate) fn scroll_page(&mut self, direction: i32, plan: &RenderPlan, steps: usize) {
        if direction == 0 {
            return;
        }

        for _ in 0..steps {
            self.scroll_by(direction, plan);
        }
    }

    pub(crate) fn scroll_home(&mut self, plan: &RenderPlan) {
        match plan.focus {
            Focus::Axes => self.axis_scroll = 0,
            Focus::Buttons => self.button_row_scroll = 0,
        }
    }

    pub(crate) fn scroll_end(&mut self, plan: &RenderPlan) {
        match plan.focus {
            Focus::Axes => self.axis_scroll = plan.scroll_bounds.axes_max,
            Focus::Buttons => self.button_row_scroll = plan.scroll_bounds.button_row_max_start,
        }
    }

    pub(crate) fn focus_next(&mut self, plan: &RenderPlan) {
        self.focus = next_focus(plan.focus, plan.focusable());
    }

    pub(crate) fn focus_prev(&mut self, plan: &RenderPlan) {
        self.focus_next(plan);
    }

    pub(crate) fn toggle_info(&mut self) {
        self.active_popup = toggled_popup(self.active_popup, ActivePopup::Info);
    }

    pub(crate) fn toggle_help(&mut self) {
        self.active_popup = toggled_popup(self.active_popup, ActivePopup::Help);
    }

    pub(crate) fn toggle_invert_y(&mut self) {
        self.joystick_invert_y = !self.joystick_invert_y;
    }
}

pub(crate) fn build_device_info_lines(
    driver_version: (u8, u8, u8),
    input_id: evdev::InputId,
    phys: Option<&str>,
    startup_warnings: &[String],
) -> Vec<String> {
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
    lines
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

fn step_scroll(offset: &mut usize, direction: i32, overflow: bool, max: usize) {
    if !overflow {
        return;
    }
    if direction < 0 {
        *offset = offset.saturating_sub(1);
    } else if direction > 0 {
        *offset = (*offset + 1).min(max);
    }
}

#[cfg(test)]
mod tests {
    use super::{ActivePopup, Focus, next_focus, toggled_popup};

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
}
