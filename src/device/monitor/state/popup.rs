use crate::device::monitor::plan::RenderPlan;

use super::{ActivePopup, Focus, MonitorState};

impl MonitorState {
    pub(crate) fn active_popup(&self) -> ActivePopup {
        self.active_popup
    }

    pub(crate) fn focus(&self) -> Focus {
        self.focus
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
