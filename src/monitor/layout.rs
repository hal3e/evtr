mod axes;
mod boxes;

use ratatui::layout::{Constraint, Layout, Rect};

pub(super) use self::{
    axes::axes_layout,
    boxes::{
        AxesPanel, ButtonsPanel, HatPanel, JoystickPanel, LayoutRequest, TouchPanel, box_layout,
        split_buttons_column,
    },
};

pub(super) fn main_layout(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area)
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::main_layout;

    #[test]
    fn main_layout_reserves_one_header_row_and_the_remaining_body() {
        let [header, body] = main_layout(Rect::new(0, 0, 20, 8));

        assert_eq!(header, Rect::new(0, 0, 20, 1));
        assert_eq!(body, Rect::new(0, 1, 20, 7));
    }
}
