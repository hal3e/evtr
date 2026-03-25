use crate::monitor::plan::NavigationContext;

use super::{ActivePopup, Focus, MonitorState};

impl MonitorState {
    pub(in crate::monitor) fn active_popup(&self) -> ActivePopup {
        self.active_popup
    }

    pub(in crate::monitor) fn focus(&self) -> Focus {
        self.focus
    }

    pub(in crate::monitor) fn focus_next(&mut self, navigation: NavigationContext) {
        self.focus = next_focus(navigation.focus(), navigation.focusable());
    }

    pub(in crate::monitor) fn focus_prev(&mut self, navigation: NavigationContext) {
        self.focus_next(navigation);
    }

    pub(in crate::monitor) fn toggle_info(&mut self) {
        self.active_popup = toggled_popup(self.active_popup, ActivePopup::Info);
    }

    pub(in crate::monitor) fn toggle_help(&mut self) {
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
    use super::{ActivePopup, Focus, MonitorState, next_focus, toggled_popup};
    use crate::monitor::plan::{Counts, NavigationContext, TestScrollBounds, TestScrollState};

    fn navigation(focus: Focus, focusable: bool) -> NavigationContext {
        NavigationContext::new_for_tests(
            focus,
            TestScrollState {
                axis: 0,
                button_row: 0,
            },
            TestScrollBounds::new_for_tests(0, 0, false, false),
            focusable,
        )
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
    fn toggle_help_closes_when_called_twice() {
        let mut state = MonitorState::new(Counts::new(1, 0, 1), Vec::new());

        state.toggle_help();
        assert_eq!(state.active_popup(), ActivePopup::Help);

        state.toggle_help();
        assert_eq!(state.active_popup(), ActivePopup::None);
    }

    #[test]
    fn toggle_info_closes_when_called_twice() {
        let mut state = MonitorState::new(Counts::new(1, 0, 1), Vec::new());

        state.toggle_info();
        assert_eq!(state.active_popup(), ActivePopup::Info);

        state.toggle_info();
        assert_eq!(state.active_popup(), ActivePopup::None);
    }

    #[test]
    fn toggle_help_and_info_replace_each_other() {
        let mut state = MonitorState::new(Counts::new(1, 0, 1), Vec::new());

        state.toggle_help();
        assert_eq!(state.active_popup(), ActivePopup::Help);

        state.toggle_info();
        assert_eq!(state.active_popup(), ActivePopup::Info);

        state.toggle_help();
        assert_eq!(state.active_popup(), ActivePopup::Help);
    }

    #[test]
    fn focus_prev_uses_the_same_cycle_behavior_as_focus_next() {
        let mut state = MonitorState::new(Counts::new(1, 0, 1), Vec::new());

        state.focus_prev(navigation(Focus::Axes, true));
        assert_eq!(state.focus(), Focus::Buttons);

        state.focus_prev(navigation(Focus::Buttons, true));
        assert_eq!(state.focus(), Focus::Axes);

        state.focus_prev(navigation(Focus::Axes, false));
        assert_eq!(state.focus(), Focus::Axes);
    }
}
