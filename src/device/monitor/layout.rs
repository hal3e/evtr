use ratatui::layout::{Constraint, Layout, Rect};

use crate::device::monitor::config;

pub(crate) fn main_layout(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area)
}

pub(crate) struct BoxLayout {
    pub(crate) axes_box: Option<Rect>,
    pub(crate) buttons_box: Option<Rect>,
}

pub(crate) struct AxesLayout {
    pub(crate) abs_area: Option<Rect>,
    pub(crate) rel_area: Option<Rect>,
}

pub(crate) fn box_layout(area: Rect, axes_present: bool, buttons_present: bool) -> BoxLayout {
    let min_axes_box = if axes_present { 2 } else { 0 };
    let min_buttons_box = if buttons_present { 1 } else { 0 };

    let mut axes_height = 0;
    let mut buttons_height = 0;

    match (axes_present, buttons_present) {
        (true, true) => {
            if area.height < min_axes_box + min_buttons_box {
                if area.height >= min_buttons_box {
                    buttons_height = area.height;
                }
            } else {
                let desired_axes = (area.height * config::AXES_BOX_PERCENT) / 100;
                let max_axes = area.height.saturating_sub(min_buttons_box);
                axes_height = desired_axes.clamp(min_axes_box, max_axes);
                buttons_height = area.height.saturating_sub(axes_height);
            }
        }
        (true, false) => {
            if area.height >= min_axes_box {
                axes_height = area.height;
            }
        }
        (false, true) => {
            if area.height >= min_buttons_box {
                buttons_height = area.height;
            }
        }
        (false, false) => {}
    }

    let axes_box = if axes_height >= min_axes_box && axes_height > 0 {
        Some(Rect::new(area.x, area.y, area.width, axes_height))
    } else {
        None
    };
    let buttons_box = if buttons_height >= min_buttons_box && buttons_height > 0 {
        Some(Rect::new(
            area.x,
            area.y + area.height.saturating_sub(buttons_height),
            area.width,
            buttons_height,
        ))
    } else {
        None
    };

    BoxLayout {
        axes_box,
        buttons_box,
    }
}

pub(crate) fn axes_layout(area: Rect, abs_count: usize, rel_count: usize) -> AxesLayout {
    let total_axes = abs_count + rel_count;
    if total_axes == 0 || area.height == 0 {
        return AxesLayout {
            abs_area: None,
            rel_area: None,
        };
    }

    if abs_count > 0 && rel_count > 0 {
        let gap = config::REL_SECTION_GAP;
        let available_for_content = area.height.saturating_sub(gap);
        let abs_portion = (available_for_content * abs_count as u16) / total_axes as u16;
        let rel_portion = available_for_content.saturating_sub(abs_portion);

        let abs_area = Rect::new(area.x, area.y, area.width, abs_portion);
        let rel_area = Rect::new(area.x, area.y + abs_portion + gap, area.width, rel_portion);

        AxesLayout {
            abs_area: Some(abs_area),
            rel_area: Some(rel_area),
        }
    } else if abs_count > 0 {
        AxesLayout {
            abs_area: Some(area),
            rel_area: None,
        }
    } else {
        AxesLayout {
            abs_area: None,
            rel_area: Some(area),
        }
    }
}
