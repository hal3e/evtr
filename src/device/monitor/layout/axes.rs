use ratatui::layout::Rect;

use crate::device::monitor::config;

pub(crate) struct AxesLayout {
    pub(crate) abs_area: Option<Rect>,
    pub(crate) rel_area: Option<Rect>,
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

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::axes_layout;

    #[test]
    fn axes_layout_splits_absolute_and_relative_sections_with_gap() {
        let layout = axes_layout(Rect::new(0, 0, 40, 10), 2, 2);

        assert_eq!(layout.abs_area, Some(Rect::new(0, 0, 40, 4)));
        assert_eq!(layout.rel_area, Some(Rect::new(0, 5, 40, 5)));
    }
}
