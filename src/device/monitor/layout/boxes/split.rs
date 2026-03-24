use ratatui::layout::Rect;

use crate::device::monitor::config;

pub(super) fn gap_if_room(width: u16, preferred_gap: u16) -> u16 {
    if width > preferred_gap * 2 {
        preferred_gap
    } else {
        0
    }
}

pub(crate) fn split_buttons_column(
    area: Rect,
    buttons_present: bool,
    main_min_width: u16,
    buttons_min_width: u16,
    min_button_gap: u16,
) -> (Rect, Option<Rect>) {
    if !buttons_present {
        return (area, None);
    }

    let gap = gap_if_room(area.width, config::MAIN_BUTTONS_GAP);
    let (main_width, buttons_width) = ratio_widths(area.width, gap, config::MAIN_COLUMN_PERCENT);

    if main_width < main_min_width || buttons_width < buttons_min_width {
        return (area, None);
    }
    if !buttons_width_ok(buttons_width, min_button_gap) {
        return (area, None);
    }

    let main_area = Rect::new(area.x, area.y, main_width, area.height);
    let buttons_area = Rect::new(
        area.x + main_width + gap,
        area.y,
        buttons_width,
        area.height,
    );
    (main_area, Some(buttons_area))
}

pub(super) fn split_row_ratio(
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    gap: u16,
    left_percent: u16,
) -> (Rect, Rect) {
    let (left_width, right_width) = ratio_widths(width, gap, left_percent);
    let left = Rect::new(x, y, left_width, height);
    let right = Rect::new(x + left_width + gap, y, right_width, height);
    (left, right)
}

pub(super) fn ratio_widths(width: u16, gap: u16, left_percent: u16) -> (u16, u16) {
    let available = width.saturating_sub(gap);
    if available < 2 {
        return (0, 0);
    }
    let left_percent = left_percent.clamp(1, 99);
    let mut left = ((available as u32).saturating_mul(left_percent as u32) / 100) as u16;
    left = left.max(1).min(available.saturating_sub(1));
    let right = available.saturating_sub(left);
    (left, right)
}

fn buttons_width_ok(width: u16, min_gap: u16) -> bool {
    if width == 0 {
        return false;
    }
    let button_width = width / config::BUTTONS_PER_ROW as u16;
    button_width > min_gap
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{ratio_widths, split_buttons_column};
    use crate::device::monitor::config;

    #[test]
    fn split_buttons_column_returns_sidebar_when_width_allows() {
        let area = Rect::new(0, 0, 100, 20);

        let (main, buttons) = split_buttons_column(
            area,
            true,
            config::MAIN_COLUMN_MIN_WIDTH,
            config::BUTTONS_COLUMN_MIN_WIDTH,
            config::BTN_COL_GAP,
        );

        let buttons = buttons.expect("expected a buttons column");
        assert_eq!(
            main.width + buttons.width + config::MAIN_BUTTONS_GAP,
            area.width
        );
        assert!(main.width >= config::MAIN_COLUMN_MIN_WIDTH);
        assert!(buttons.width >= config::BUTTONS_COLUMN_MIN_WIDTH);
    }

    #[test]
    fn split_buttons_column_stays_single_column_when_sidebar_is_too_narrow() {
        let area = Rect::new(0, 0, 50, 20);

        let (main, buttons) = split_buttons_column(
            area,
            true,
            config::MAIN_COLUMN_MIN_WIDTH,
            config::BUTTONS_COLUMN_MIN_WIDTH,
            config::BTN_COL_GAP,
        );

        assert_eq!(main, area);
        assert!(buttons.is_none());
    }

    #[test]
    fn ratio_widths_keeps_both_columns_nonzero() {
        assert_eq!(ratio_widths(10, 0, 99), (9, 1));
        assert_eq!(ratio_widths(10, 0, 1), (1, 9));
    }
}
