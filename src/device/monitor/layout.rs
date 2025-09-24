use ratatui::layout::{Constraint, Layout, Rect};

use crate::device::monitor::config;

pub(crate) fn main_layout(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area)
}

pub(crate) struct SectionSizer {
    pub(crate) abs_area: Option<Rect>,
    pub(crate) rel_area: Option<Rect>,
    pub(crate) btn_area: Option<Rect>,
}

impl SectionSizer {
    pub(crate) fn new(area: Rect, btn_rows: usize, abs_count: usize, rel_count: usize) -> Self {
        // If the area is too narrow to render axes at all, allocate the entire
        // content height to the button section only.
        let axes_width_ok = area.width >= config::AXIS_MIN_WIDTH;
        let abs_count = if axes_width_ok { abs_count } else { 0 };
        let rel_count = if axes_width_ok { rel_count } else { 0 };
        let total_axes = abs_count + rel_count;
        let (min_axes_height, btn_height_full) =
            section_min_heights(btn_rows, abs_count, rel_count);

        // Only buttons present: put them at the top, empty space below.
        if total_axes == 0 {
            let h = btn_height_full.min(area.height);
            let btn_area = if h > 0 {
                Some(Rect::new(area.x, area.y, area.width, h))
            } else {
                None
            };
            return Self {
                abs_area: None,
                rel_area: None,
                btn_area,
            };
        }

        // Prefer to size axes so a full button section could fit beneath, if space allows.
        let target_axes_available = if btn_rows > 0 {
            area.height.saturating_sub(btn_height_full)
        } else {
            area.height
        };
        let axes_height = optimal_axes_height(
            min_axes_height,
            target_axes_available,
            total_axes,
            abs_count > 0 && rel_count > 0,
        );

        let axes_area = Rect::new(area.x, area.y, area.width, axes_height);

        // Place buttons immediately after axes; clamp to remaining height.
        let remaining_after_axes = area.height.saturating_sub(axes_height);
        let btn_h = if btn_rows > 0 {
            btn_height_full.min(remaining_after_axes)
        } else {
            0
        };
        let btn_area = if btn_h > 0 {
            Some(Rect::new(area.x, area.y + axes_height, area.width, btn_h))
        } else {
            None
        };

        // Split between abs and rel if both present
        if abs_count > 0 && rel_count > 0 {
            let gap = config::REL_SECTION_GAP;
            let available_for_content = axes_height.saturating_sub(gap);
            let abs_portion = (available_for_content * abs_count as u16) / total_axes as u16;
            let rel_portion = available_for_content.saturating_sub(abs_portion);

            let abs_area = Rect::new(axes_area.x, axes_area.y, axes_area.width, abs_portion);
            let rel_area = Rect::new(
                axes_area.x,
                axes_area.y + abs_portion + gap,
                axes_area.width,
                rel_portion,
            );

            Self {
                abs_area: Some(abs_area),
                rel_area: Some(rel_area),
                btn_area,
            }
        } else if abs_count > 0 {
            Self {
                abs_area: Some(axes_area),
                rel_area: None,
                btn_area,
            }
        } else {
            Self {
                abs_area: None,
                rel_area: Some(axes_area),
                btn_area,
            }
        }
    }
}

fn optimal_axes_height(
    min_height: u16,
    available_height: u16,
    total_axes: usize,
    has_rel_section: bool,
) -> u16 {
    if total_axes == 0 || available_height <= min_height {
        return min_height.min(available_height);
    }

    let rel_gap = if has_rel_section {
        config::REL_SECTION_GAP
    } else {
        0
    };
    for &bar_height in &config::BAR_HEIGHTS {
        let total_needed = (total_axes as u16 * (bar_height + config::AXIS_GAP)) + rel_gap;
        if total_needed <= available_height {
            return total_needed;
        }
    }

    min_height.min(available_height)
}

pub(crate) fn section_min_heights(
    btn_rows: usize,
    abs_count: usize,
    rel_count: usize,
) -> (u16, u16) {
    let total_axes = abs_count + rel_count;
    if total_axes == 0 {
        let btn_height = if btn_rows == 0 {
            0
        } else {
            (btn_rows as u16 * config::BUTTON_HEIGHT) + config::BTN_SECTION_VERT_PADDING
        };
        return (0, btn_height);
    }
    let has_rel_section = abs_count > 0 && rel_count > 0;
    let rel_title_height = if has_rel_section {
        config::REL_SECTION_GAP
    } else {
        0
    };
    let min_axes_height = (total_axes as u16 * (1 + config::AXIS_GAP)) + rel_title_height;
    let btn_height = if btn_rows == 0 {
        0
    } else {
        (btn_rows as u16 * config::BUTTON_HEIGHT) + config::BTN_SECTION_VERT_PADDING
    };
    (min_axes_height, btn_height)
}
