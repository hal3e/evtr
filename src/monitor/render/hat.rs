use ratatui::{
    buffer::Buffer,
    layout::Rect,
    symbols::Marker,
    widgets::{
        Widget,
        canvas::{Canvas, Points},
    },
};

use crate::monitor::{config, view_model::HatState};

use super::geometry::{coord_from_index, fit_centered_aspect_rect, inset_rect};

pub(crate) struct HatRenderer;

impl HatRenderer {
    pub(crate) fn render(area: Rect, state: HatState, buf: &mut Buffer) {
        let Some(square) = fit_centered_aspect_rect(area, config::JOYSTICK_ASPECT_RATIO) else {
            return;
        };
        let square = inset_rect(square, config::HAT_PADDING);
        if square.width < 2 || square.height < 2 {
            return;
        }

        let grid_width = square.width as usize;
        let grid_height = square.height as usize * 2;
        if grid_width < 2 || grid_height < 2 {
            return;
        }

        let blocks = config::HAT_BLOCKS.max(1);
        let thickness = config::HAT_THICKNESS.max(1);
        let base_points = base_points(grid_width, grid_height, blocks, thickness);
        let active_points = active_points(state, grid_width, grid_height, blocks, thickness);

        Canvas::default()
            .marker(Marker::HalfBlock)
            .x_bounds([-1.0, 1.0])
            .y_bounds([-1.0, 1.0])
            .paint(|ctx| {
                ctx.draw(&Points::new(&base_points, config::COLOR_TOUCH_INACTIVE));
                if !active_points.is_empty() {
                    ctx.draw(&Points::new(&active_points, config::COLOR_TOUCH_POINT));
                }
            })
            .render(square, buf);
    }
}

fn base_points(
    grid_width: usize,
    grid_height: usize,
    blocks: usize,
    thickness: usize,
) -> Vec<(f64, f64)> {
    let blocks_x = blocks.min(grid_width);
    let blocks_y = blocks.min(grid_height);
    let thickness_x = thickness.min(grid_width);
    let thickness_y = thickness.min(grid_height);
    let mut points = Vec::with_capacity(2 * blocks_x * thickness_y + 2 * blocks_y * thickness_x);

    let x_left = edge_indices(grid_width, blocks_x, Edge::Start);
    let x_right = edge_indices(grid_width, blocks_x, Edge::End);
    let y_up = edge_indices(grid_height, blocks_y, Edge::End);
    let y_down = edge_indices(grid_height, blocks_y, Edge::Start);
    let y_thickness = centered_indices(grid_height, thickness_y);
    let x_thickness = centered_indices(grid_width, thickness_x);

    for x in x_left {
        let x_coord = coord_from_index(x, grid_width);
        for &y in &y_thickness {
            points.push((x_coord, coord_from_index(y, grid_height)));
        }
    }
    for x in x_right {
        let x_coord = coord_from_index(x, grid_width);
        for &y in &y_thickness {
            points.push((x_coord, coord_from_index(y, grid_height)));
        }
    }
    for y in y_up {
        let y_coord = coord_from_index(y, grid_height);
        for &x in &x_thickness {
            points.push((coord_from_index(x, grid_width), y_coord));
        }
    }
    for y in y_down {
        let y_coord = coord_from_index(y, grid_height);
        for &x in &x_thickness {
            points.push((coord_from_index(x, grid_width), y_coord));
        }
    }

    points
}

fn active_points(
    state: HatState,
    grid_width: usize,
    grid_height: usize,
    blocks: usize,
    thickness: usize,
) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    if state.x == 0 && state.y == 0 {
        return points;
    }

    let blocks_x = blocks.min(grid_width);
    let blocks_y = blocks.min(grid_height);
    let thickness_x = thickness.min(grid_width);
    let thickness_y = thickness.min(grid_height);
    let y_thickness = centered_indices(grid_height, thickness_y);
    let x_thickness = centered_indices(grid_width, thickness_x);

    if state.x != 0 {
        let edge = if state.x < 0 { Edge::Start } else { Edge::End };
        for x in edge_indices(grid_width, blocks_x, edge) {
            let x_coord = coord_from_index(x, grid_width);
            for &y in &y_thickness {
                points.push((x_coord, coord_from_index(y, grid_height)));
            }
        }
    }

    if state.y != 0 {
        let edge = if state.y > 0 { Edge::End } else { Edge::Start };
        for y in edge_indices(grid_height, blocks_y, edge) {
            let y_coord = coord_from_index(y, grid_height);
            for &x in &x_thickness {
                points.push((coord_from_index(x, grid_width), y_coord));
            }
        }
    }

    points
}

#[derive(Clone, Copy)]
enum Edge {
    Start,
    End,
}

fn edge_indices(total: usize, count: usize, edge: Edge) -> Vec<usize> {
    let count = count.min(total);
    let start = match edge {
        Edge::Start => 0,
        Edge::End => total.saturating_sub(count),
    };
    (start..start.saturating_add(count)).collect()
}

fn centered_indices(total: usize, count: usize) -> Vec<usize> {
    let count = count.min(total).max(1);
    let start = (total.saturating_sub(count)) / 2;
    (start..start.saturating_add(count)).collect()
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect};

    use super::{Edge, HatRenderer, active_points, centered_indices, edge_indices};
    use crate::monitor::{model::AbsoluteAxis, view_model::HatState};

    fn non_blank_cells(buf: &Buffer) -> usize {
        buf.content()
            .iter()
            .filter(|cell| !cell.symbol().trim().is_empty())
            .count()
    }

    #[test]
    fn active_points_is_empty_for_neutral_hat_state() {
        let state = HatState::from_axes(
            AbsoluteAxis {
                min: -1,
                max: 1,
                value: 0,
            },
            AbsoluteAxis {
                min: -1,
                max: 1,
                value: 0,
            },
            false,
        );

        assert!(active_points(state, 8, 8, 4, 2).is_empty());
    }

    #[test]
    fn edge_indices_cover_requested_start_and_end_ranges() {
        assert_eq!(edge_indices(6, 2, Edge::Start), vec![0, 1]);
        assert_eq!(edge_indices(6, 2, Edge::End), vec![4, 5]);
    }

    #[test]
    fn centered_indices_handles_even_odd_and_clamped_counts() {
        assert_eq!(centered_indices(7, 3), vec![2, 3, 4]);
        assert_eq!(centered_indices(6, 2), vec![2, 3]);
        assert_eq!(centered_indices(3, 10), vec![0, 1, 2]);
    }

    #[test]
    fn render_returns_without_touching_a_too_small_area() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        let state = HatState::from_axes(
            AbsoluteAxis {
                min: -1,
                max: 1,
                value: 1,
            },
            AbsoluteAxis {
                min: -1,
                max: 1,
                value: 0,
            },
            false,
        );

        HatRenderer::render(area, state, &mut buf);

        assert_eq!(non_blank_cells(&buf), 0);
    }
}
