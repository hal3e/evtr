use crate::monitor::plan::NavigationContext;

use super::{Focus, MonitorState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScrollCursor {
    axis: usize,
    button_row: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScrollLimits {
    axes_max: usize,
    button_row_max_start: usize,
    axes_overflow: bool,
    buttons_overflow: bool,
}

impl ScrollCursor {
    fn from_state(state: &MonitorState) -> Self {
        Self {
            axis: state.axis_scroll,
            button_row: state.button_row_scroll,
        }
    }

    fn from_navigation(navigation: NavigationContext) -> Self {
        Self {
            axis: navigation.scroll().axis,
            button_row: navigation.scroll().button_row,
        }
    }

    fn apply(self, state: &mut MonitorState) {
        state.axis_scroll = self.axis;
        state.button_row_scroll = self.button_row;
    }

    fn step(self, focus: Focus, direction: i32, limits: ScrollLimits) -> Self {
        if direction == 0 {
            return self;
        }

        match focus {
            Focus::Axes => Self {
                axis: step_offset(self.axis, direction, limits.axes_overflow, limits.axes_max),
                ..self
            },
            Focus::Buttons => Self {
                button_row: step_offset(
                    self.button_row,
                    direction,
                    limits.buttons_overflow,
                    limits.button_row_max_start,
                ),
                ..self
            },
        }
    }

    fn home(self, focus: Focus) -> Self {
        match focus {
            Focus::Axes => Self { axis: 0, ..self },
            Focus::Buttons => Self {
                button_row: 0,
                ..self
            },
        }
    }

    fn end(self, focus: Focus, limits: ScrollLimits) -> Self {
        match focus {
            Focus::Axes => Self {
                axis: limits.axes_max,
                ..self
            },
            Focus::Buttons => Self {
                button_row: limits.button_row_max_start,
                ..self
            },
        }
    }
}

impl ScrollLimits {
    fn from_navigation(navigation: NavigationContext) -> Self {
        let scroll_bounds = navigation.scroll_bounds();
        Self {
            axes_max: scroll_bounds.axes_max,
            button_row_max_start: scroll_bounds.button_row_max_start,
            axes_overflow: scroll_bounds.axes_overflow,
            buttons_overflow: scroll_bounds.buttons_overflow,
        }
    }
}

impl MonitorState {
    pub(in crate::monitor) fn axis_scroll(&self) -> usize {
        self.axis_scroll
    }

    pub(in crate::monitor) fn button_row_scroll(&self) -> usize {
        self.button_row_scroll
    }

    pub(in crate::monitor) fn sync_from_navigation(&mut self, navigation: NavigationContext) {
        self.focus = navigation.focus();
        ScrollCursor::from_navigation(navigation).apply(self);
    }

    pub(in crate::monitor) fn scroll_by(&mut self, direction: i32, navigation: NavigationContext) {
        let cursor = ScrollCursor::from_state(self).step(
            navigation.focus(),
            direction,
            ScrollLimits::from_navigation(navigation),
        );
        cursor.apply(self);
    }

    pub(in crate::monitor) fn scroll_page(
        &mut self,
        direction: i32,
        navigation: NavigationContext,
        steps: usize,
    ) {
        if direction == 0 {
            return;
        }

        let limits = ScrollLimits::from_navigation(navigation);
        let mut cursor = ScrollCursor::from_state(self);
        for _ in 0..steps {
            cursor = cursor.step(navigation.focus(), direction, limits);
        }
        cursor.apply(self);
    }

    pub(in crate::monitor) fn scroll_home(&mut self, navigation: NavigationContext) {
        ScrollCursor::from_state(self)
            .home(navigation.focus())
            .apply(self);
    }

    pub(in crate::monitor) fn scroll_end(&mut self, navigation: NavigationContext) {
        ScrollCursor::from_state(self)
            .end(
                navigation.focus(),
                ScrollLimits::from_navigation(navigation),
            )
            .apply(self);
    }
}

fn step_offset(current: usize, direction: i32, overflow: bool, max: usize) -> usize {
    if !overflow {
        return current;
    }
    if direction < 0 {
        current.saturating_sub(1)
    } else if direction > 0 {
        (current + 1).min(max)
    } else {
        current
    }
}

#[cfg(test)]
mod tests {
    use super::{Focus, MonitorState, ScrollCursor, ScrollLimits, step_offset};
    use crate::monitor::plan::{Counts, NavigationContext, TestScrollBounds, TestScrollState};

    struct NavSpec {
        focus: Focus,
        axis_scroll: usize,
        button_row_scroll: usize,
        axes_max: usize,
        button_row_max_start: usize,
        axes_overflow: bool,
        buttons_overflow: bool,
        focusable: bool,
    }

    fn navigation(spec: NavSpec) -> NavigationContext {
        NavigationContext::new_for_tests(
            spec.focus,
            TestScrollState {
                axis: spec.axis_scroll,
                button_row: spec.button_row_scroll,
            },
            TestScrollBounds::new_for_tests(
                spec.axes_max,
                spec.button_row_max_start,
                spec.axes_overflow,
                spec.buttons_overflow,
            ),
            spec.focusable,
        )
    }

    #[test]
    fn step_offset_moves_only_when_overflow_is_enabled() {
        assert_eq!(step_offset(2, 1, false, 10), 2);
        assert_eq!(step_offset(2, 1, true, 10), 3);
        assert_eq!(step_offset(0, -1, true, 10), 0);
    }

    #[test]
    fn scroll_cursor_end_only_updates_the_focused_target() {
        let cursor = ScrollCursor {
            axis: 1,
            button_row: 2,
        };
        let limits = ScrollLimits {
            axes_max: 7,
            button_row_max_start: 5,
            axes_overflow: true,
            buttons_overflow: true,
        };

        assert_eq!(
            cursor.end(Focus::Axes, limits),
            ScrollCursor {
                axis: 7,
                button_row: 2,
            }
        );
        assert_eq!(
            cursor.end(Focus::Buttons, limits),
            ScrollCursor {
                axis: 1,
                button_row: 5,
            }
        );
    }

    #[test]
    fn scroll_cursor_home_only_updates_the_focused_target() {
        let cursor = ScrollCursor {
            axis: 3,
            button_row: 4,
        };

        assert_eq!(
            cursor.home(Focus::Axes),
            ScrollCursor {
                axis: 0,
                button_row: 4,
            }
        );
        assert_eq!(
            cursor.home(Focus::Buttons),
            ScrollCursor {
                axis: 3,
                button_row: 0,
            }
        );
    }

    #[test]
    fn scroll_cursor_step_only_updates_the_focused_target() {
        let cursor = ScrollCursor {
            axis: 1,
            button_row: 2,
        };
        let limits = ScrollLimits {
            axes_max: 4,
            button_row_max_start: 5,
            axes_overflow: true,
            buttons_overflow: true,
        };

        assert_eq!(
            cursor.step(Focus::Axes, 1, limits),
            ScrollCursor {
                axis: 2,
                button_row: 2,
            }
        );
        assert_eq!(
            cursor.step(Focus::Buttons, 1, limits),
            ScrollCursor {
                axis: 1,
                button_row: 3,
            }
        );
    }

    #[test]
    fn scroll_page_zero_is_a_no_op() {
        let mut state = MonitorState::new(Counts::new(1, 0, 1), Vec::new());
        state.axis_scroll = 2;
        state.button_row_scroll = 3;

        state.scroll_page(
            0,
            navigation(NavSpec {
                focus: Focus::Axes,
                axis_scroll: 2,
                button_row_scroll: 3,
                axes_max: 7,
                button_row_max_start: 5,
                axes_overflow: true,
                buttons_overflow: true,
                focusable: true,
            }),
            10,
        );

        assert_eq!(state.axis_scroll(), 2);
        assert_eq!(state.button_row_scroll(), 3);
    }

    #[test]
    fn sync_from_navigation_updates_focus_and_scroll_positions() {
        let mut state = MonitorState::new(Counts::new(1, 0, 1), Vec::new());

        state.sync_from_navigation(navigation(NavSpec {
            focus: Focus::Buttons,
            axis_scroll: 4,
            button_row_scroll: 2,
            axes_max: 7,
            button_row_max_start: 5,
            axes_overflow: true,
            buttons_overflow: true,
            focusable: true,
        }));

        assert_eq!(state.focus, Focus::Buttons);
        assert_eq!(state.axis_scroll(), 4);
        assert_eq!(state.button_row_scroll(), 2);
    }
}
