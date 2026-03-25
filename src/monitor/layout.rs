mod axes;
mod boxes;

use ratatui::layout::{Constraint, Layout, Rect};

pub(crate) use self::{
    axes::axes_layout,
    boxes::{
        AxesPanel, ButtonsPanel, HatPanel, JoystickPanel, LayoutRequest, TouchPanel, box_layout,
        split_buttons_column,
    },
};

pub(crate) fn main_layout(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area)
}
