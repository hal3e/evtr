use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, Borders, Widget},
};

pub(crate) fn styled_titled_block<'a>(
    title: &'a str,
    style: Style,
    title_alignment: Alignment,
) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(title_alignment)
        .style(style)
}

pub(crate) fn bordered_titled_block<'a>(
    title: &'a str,
    border_style: Style,
    title_alignment: Alignment,
) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(title_alignment)
        .border_style(border_style)
}

pub(crate) fn render_bordered_titled_box(
    area: Rect,
    title: &str,
    border_style: Style,
    title_alignment: Alignment,
    buf: &mut Buffer,
) -> Rect {
    if area.height < 2 || area.width < 2 {
        return area;
    }

    let block = bordered_titled_block(title, border_style, title_alignment);
    let inner = bordered_box_inner(area);
    block.render(area, buf);
    inner
}

pub(crate) fn bordered_box_inner(area: Rect) -> Rect {
    if area.height >= 2 && area.width >= 2 {
        Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2)
    } else {
        area
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{
        buffer::Buffer,
        layout::{Alignment, Rect},
        style::Style,
    };

    use super::{bordered_box_inner, render_bordered_titled_box};

    #[test]
    fn render_bordered_titled_box_returns_inner_rect_for_renderable_area() {
        let area = Rect::new(0, 0, 8, 4);
        let mut buf = Buffer::empty(area);

        let inner =
            render_bordered_titled_box(area, " Box ", Style::default(), Alignment::Left, &mut buf);

        assert_eq!(inner, Rect::new(1, 1, 6, 2));
    }

    #[test]
    fn render_bordered_titled_box_returns_original_area_when_too_small() {
        let area = Rect::new(0, 0, 1, 4);
        let mut buf = Buffer::empty(area);

        let inner =
            render_bordered_titled_box(area, " Box ", Style::default(), Alignment::Left, &mut buf);

        assert_eq!(inner, area);
    }

    #[test]
    fn bordered_box_inner_shrinks_by_the_border_width() {
        assert_eq!(
            bordered_box_inner(Rect::new(0, 0, 8, 4)),
            Rect::new(1, 1, 6, 2)
        );
        assert_eq!(
            bordered_box_inner(Rect::new(0, 0, 1, 4)),
            Rect::new(0, 0, 1, 4)
        );
    }
}
