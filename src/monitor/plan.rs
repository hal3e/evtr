mod areas;
mod scroll;

use ratatui::layout::Rect;

use super::{
    config,
    layout::main_layout,
    model::InputCollection,
    render::{hat::HatState, joystick::JoystickState},
    state::{Focus, MonitorState},
    touch::TouchState,
};

use self::{
    areas::{PlannedAreas, PlannedBoxes, WidgetState, plan_areas},
    scroll::{ScrollBounds, ScrollState, VisibleCapacities, clamp_scroll_state},
};

#[derive(Clone, Copy)]
pub(crate) struct Counts {
    abs: usize,
    rel: usize,
    btn: usize,
}

impl Counts {
    pub(crate) fn new(abs: usize, rel: usize, btn: usize) -> Self {
        Self { abs, rel, btn }
    }

    pub(crate) fn total_axes(&self) -> usize {
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

pub(crate) struct RenderPlan {
    pub(crate) focus: Focus,
    pub(crate) scroll: ScrollState,
    pub(crate) effective_counts: Counts,
    pub(crate) scroll_bounds: ScrollBounds,
    pub(crate) boxes: PlannedBoxes,
    pub(crate) areas: PlannedAreas,
    pub(crate) joystick: JoystickState,
    pub(crate) hat_state: Option<HatState>,
}

impl RenderPlan {
    pub(crate) fn focusable(&self) -> bool {
        self.boxes.axes.is_some() && self.boxes.buttons.is_some()
    }

    pub(crate) fn axis_offsets(&self) -> (usize, usize) {
        self.scroll_bounds
            .axis_offsets(self.effective_counts, self.scroll.axis)
    }
}

pub(crate) fn build_render_plan(
    area: Rect,
    state: &MonitorState,
    inputs: &InputCollection,
    touch: &TouchState,
) -> RenderPlan {
    let counts = state.counts();
    let [_, content] = main_layout(area);
    let min_button_gap = config::BTN_COL_GAP.max(config::COMPACT_BTN_COL_GAP);
    let widget_state = WidgetState::from_inputs(state, counts, inputs, touch);
    let area_plan = plan_areas(
        content,
        counts,
        min_button_gap,
        state.focus(),
        &widget_state,
    );
    let capacities = VisibleCapacities::from_areas(counts, &area_plan.areas);

    let abs_visible = capacities.abs > 0;
    let rel_visible = capacities.rel > 0;
    let buttons_visible = capacities.button_rows > 0;

    let effective_counts = counts.filtered(abs_visible, rel_visible, buttons_visible);
    let scroll_bounds = ScrollBounds::from_capacities(
        effective_counts,
        capacities.abs,
        capacities.rel,
        capacities.button_rows,
    );
    let scroll = clamp_scroll_state(state, &scroll_bounds, &capacities);

    RenderPlan {
        focus: area_plan.focus,
        scroll,
        effective_counts,
        scroll_bounds,
        boxes: area_plan.boxes,
        areas: area_plan.areas,
        joystick: widget_state.joystick,
        hat_state: widget_state.hat_state,
    }
}

#[cfg(test)]
mod tests {
    use super::Counts;

    #[test]
    fn counts_total_axes_tracks_absolute_and_relative_inputs() {
        assert_eq!(Counts::new(2, 3, 4).total_axes(), 5);
    }
}
