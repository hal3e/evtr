use crate::device::monitor::plan::RenderPlan;

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

    fn from_plan(plan: &RenderPlan) -> Self {
        Self {
            axis: plan.scroll.axis,
            button_row: plan.scroll.button_row,
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
    fn from_plan(plan: &RenderPlan) -> Self {
        Self {
            axes_max: plan.scroll_bounds.axes_max,
            button_row_max_start: plan.scroll_bounds.button_row_max_start,
            axes_overflow: plan.scroll_bounds.axes_overflow,
            buttons_overflow: plan.scroll_bounds.buttons_overflow,
        }
    }
}

impl MonitorState {
    pub(crate) fn axis_scroll(&self) -> usize {
        self.axis_scroll
    }

    pub(crate) fn button_row_scroll(&self) -> usize {
        self.button_row_scroll
    }

    pub(crate) fn sync_from_plan(&mut self, plan: &RenderPlan) {
        self.focus = plan.focus;
        ScrollCursor::from_plan(plan).apply(self);
    }

    pub(crate) fn scroll_by(&mut self, direction: i32, plan: &RenderPlan) {
        let cursor = ScrollCursor::from_state(self).step(
            plan.focus,
            direction,
            ScrollLimits::from_plan(plan),
        );
        cursor.apply(self);
    }

    pub(crate) fn scroll_page(&mut self, direction: i32, plan: &RenderPlan, steps: usize) {
        if direction == 0 {
            return;
        }

        let limits = ScrollLimits::from_plan(plan);
        let mut cursor = ScrollCursor::from_state(self);
        for _ in 0..steps {
            cursor = cursor.step(plan.focus, direction, limits);
        }
        cursor.apply(self);
    }

    pub(crate) fn scroll_home(&mut self, plan: &RenderPlan) {
        ScrollCursor::from_state(self).home(plan.focus).apply(self);
    }

    pub(crate) fn scroll_end(&mut self, plan: &RenderPlan) {
        ScrollCursor::from_state(self)
            .end(plan.focus, ScrollLimits::from_plan(plan))
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
    use super::{Focus, ScrollCursor, ScrollLimits, step_offset};

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
}
