use ratatui::layout::Rect;

use super::Popup;
use crate::device::text;

pub(super) fn popup_area<S: AsRef<str>>(
    area: Rect,
    popup: &Popup<'_>,
    lines: &[S],
) -> Option<Rect> {
    if area.width < popup.min_width || area.height < popup.min_height {
        return None;
    }

    let max_width = popup.max_width.unwrap_or(area.width).min(area.width);
    let max_height = popup.max_height.unwrap_or(area.height).min(area.height);
    if max_width == 0 || max_height == 0 {
        return None;
    }

    let desired_width = popup_width(popup, lines, max_width);
    let desired_height = popup_height(popup, lines, desired_width, max_height);
    let x = area.x + (area.width.saturating_sub(desired_width)) / 2;
    let y = area.y + (area.height.saturating_sub(desired_height)) / 2;
    Some(Rect::new(x, y, desired_width, desired_height))
}

fn popup_width<S: AsRef<str>>(popup: &Popup<'_>, lines: &[S], max_width: u16) -> u16 {
    let max_line = lines
        .iter()
        .map(|line| text::display_width(line.as_ref()))
        .max()
        .unwrap_or(0) as u16;
    max_line.saturating_add(2).clamp(popup.min_width, max_width)
}

fn popup_height<S: AsRef<str>>(popup: &Popup<'_>, lines: &[S], width: u16, max_height: u16) -> u16 {
    let text_width = width.saturating_sub(2).max(1) as usize;
    let wrapped_lines: usize = lines
        .iter()
        .map(|line| {
            let line_width = text::display_width(line.as_ref());
            line_width.max(1).div_ceil(text_width)
        })
        .sum();
    (wrapped_lines as u16 + 2).clamp(popup.min_height, max_height)
}

#[cfg(test)]
mod tests {
    use ratatui::{
        layout::{Alignment, Rect},
        style::Style,
        widgets::Wrap,
    };

    use super::{popup_area, popup_height, popup_width};
    use crate::device::popup::Popup;

    fn popup() -> Popup<'static> {
        Popup::new("Info")
            .min_size(3, 3)
            .text_style(Style::default())
            .border_style(Style::default())
            .text_alignment(Alignment::Left)
            .title_alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
    }

    #[test]
    fn popup_width_uses_display_width() {
        let lines = ["界界界"];
        let popup = popup();

        assert_eq!(popup_width(&popup, &lines, 20), 8);
    }

    #[test]
    fn popup_height_wraps_by_display_width() {
        let lines = ["界界界"];
        let popup = popup();

        assert_eq!(popup_height(&popup, &lines, 6, 20), 4);
    }

    #[test]
    fn popup_area_centers_measured_size() {
        let lines = ["abc"];
        let popup = popup();

        assert_eq!(
            popup_area(Rect::new(0, 0, 9, 7), &popup, &lines),
            Some(Rect::new(2, 2, 5, 3))
        );
    }
}
