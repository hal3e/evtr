mod areas;
mod scroll;

use ratatui::layout::Rect;

use super::{
    config,
    layout::main_layout,
    state::{Focus, MonitorState},
    view_model::MonitorViewModel,
};

use self::{
    areas::{PlannedAreas, PlannedBoxes, plan_areas},
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

    pub(crate) fn has_buttons(&self) -> bool {
        self.btn > 0
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NavigationContext {
    focus: Focus,
    scroll: ScrollState,
    scroll_bounds: ScrollBounds,
    focusable: bool,
}

impl NavigationContext {
    pub(crate) fn focus(self) -> Focus {
        self.focus
    }

    pub(crate) fn scroll(self) -> ScrollState {
        self.scroll
    }

    pub(crate) fn scroll_bounds(self) -> ScrollBounds {
        self.scroll_bounds
    }

    pub(crate) fn focusable(self) -> bool {
        self.focusable
    }
}

impl RenderPlan {
    pub(crate) fn focusable(&self) -> bool {
        self.boxes.axes.is_some() && self.boxes.buttons.is_some()
    }

    pub(crate) fn navigation_context(&self) -> NavigationContext {
        NavigationContext {
            focus: self.focus,
            scroll: self.scroll,
            scroll_bounds: self.scroll_bounds,
            focusable: self.focusable(),
        }
    }

    pub(crate) fn axis_offsets(&self) -> (usize, usize) {
        self.scroll_bounds
            .axis_offsets(self.effective_counts, self.scroll.axis)
    }
}

pub(crate) fn build_render_plan(
    area: Rect,
    state: &MonitorState,
    view_model: &MonitorViewModel,
) -> RenderPlan {
    let counts = state.counts();
    let [_, content] = main_layout(area);
    let min_button_gap = config::BTN_COL_GAP.max(config::COMPACT_BTN_COL_GAP);
    let area_plan = plan_areas(content, counts, min_button_gap, state.focus(), view_model);
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
    }
}

#[cfg(test)]
mod tests {
    use super::{Counts, NavigationContext, RenderPlan};
    use crate::monitor::{
        plan::{
            areas::{PlannedAreas, PlannedBoxes},
            scroll::{ScrollBounds, ScrollState},
        },
        state::Focus,
    };

    #[test]
    fn counts_total_axes_tracks_absolute_and_relative_inputs() {
        assert_eq!(Counts::new(2, 3, 4).total_axes(), 5);
    }

    #[test]
    fn navigation_context_captures_navigation_fields_only() {
        let scroll_bounds = ScrollBounds::from_capacities(Counts::new(4, 5, 6), 2, 3, 4);
        let plan = RenderPlan {
            focus: Focus::Buttons,
            scroll: ScrollState {
                axis: 2,
                button_row: 3,
            },
            effective_counts: Counts::new(4, 5, 6),
            scroll_bounds,
            boxes: PlannedBoxes {
                joystick: None,
                hat: None,
                axes: Some(ratatui::layout::Rect::default()),
                touch: None,
                buttons: Some(ratatui::layout::Rect::default()),
            },
            areas: PlannedAreas {
                joystick: None,
                hat: None,
                abs: None,
                rel: None,
                touch: None,
                buttons: None,
            },
        };

        let navigation = plan.navigation_context();

        assert_eq!(
            navigation,
            NavigationContext {
                focus: Focus::Buttons,
                scroll: ScrollState {
                    axis: 2,
                    button_row: 3,
                },
                scroll_bounds,
                focusable: true,
            }
        );
    }
}
