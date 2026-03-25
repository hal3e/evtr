use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{List, ListItem, ListState, Paragraph, Widget, Wrap},
};

use super::{commands::SelectorMode, state::SelectorState};
use crate::ui::{
    popup::{error_popup, help_popup, render_popup},
    theme, widgets,
};

const LAYOUT_MARGIN_PCT: u16 = 20;
const LAYOUT_CONTENT_WIDTH_PCT: u16 = 60;
const TOP_PADDING_HEIGHT: u16 = 1;
const SEARCH_BOX_HEIGHT: u16 = 3;
const MAIN_MIN_HEIGHT: u16 = 3;
const POPUP_MIN_WIDTH: u16 = 10;
const POPUP_MIN_HEIGHT: u16 = 3;
const POPUP_MAX_WIDTH: u16 = 80;
const HELP_POPUP_MIN_WIDTH: u16 = 30;
const HELP_POPUP_MIN_HEIGHT: u16 = 6;
const HELP_POPUP_MAX_WIDTH: u16 = 80;
const HELP_LINES: &[&str] = &[
    "Move: Up/Down, Ctrl-P/Ctrl-N, PageUp/PageDown, Home/End",
    "Select: Enter",
    "Exit: Esc or Ctrl-C",
    "Search: type to filter, Backspace, Ctrl-U clear",
    "Refresh: Ctrl-R",
    "Help: ? (press ? or Esc to close)",
];

pub(crate) fn render_selector(state: &SelectorState, area: Rect, buf: &mut Buffer) {
    use Constraint::{Length, Min, Percentage};

    let [_left_margin, content_area, _right_margin] = Layout::horizontal([
        Percentage(LAYOUT_MARGIN_PCT),
        Percentage(LAYOUT_CONTENT_WIDTH_PCT),
        Percentage(LAYOUT_MARGIN_PCT),
    ])
    .areas(area);

    let [_top_padding, search_area, list_area] = Layout::vertical([
        Length(TOP_PADDING_HEIGHT),
        Length(SEARCH_BOX_HEIGHT),
        Min(MAIN_MIN_HEIGHT),
    ])
    .areas(content_area);

    render_search_box(state, search_area, buf);
    render_device_list(state, list_area, buf);
    render_help_popup(state, area, buf);
    render_error_popup(state, area, buf);
}

fn render_search_box(state: &SelectorState, area: Rect, buf: &mut Buffer) {
    let search_text = format!(" {}_", state.search_query());
    Paragraph::new(search_text)
        .block(widgets::accent_titled_block(" Search "))
        .style(theme::style_text())
        .render(area, buf);
}

fn render_device_list(state: &SelectorState, area: Rect, buf: &mut Buffer) {
    if state.filtered_indexes().is_empty() {
        Paragraph::new(empty_state_message(state.search_query()))
            .block(widgets::accent_titled_block(" Devices "))
            .style(theme::style_text())
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(area, buf);
        return;
    }

    let items = state.filtered_indexes().iter().filter_map(|&device_index| {
        state
            .device_identifier(device_index)
            .map(|identifier| ListItem::new(identifier.to_string()))
    });

    let list = List::new(items)
        .block(widgets::accent_titled_block(" Devices "))
        .style(theme::style_text())
        .highlight_style(Style::default().bg(theme::MUTED_COLOR))
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_filtered_index()));

    ratatui::widgets::StatefulWidget::render(list, area, buf, &mut list_state);
}

fn render_error_popup(state: &SelectorState, area: Rect, buf: &mut Buffer) {
    let Some(message) = state.error_message() else {
        return;
    };
    let lines = [message];
    let popup = error_popup(
        " Error ",
        POPUP_MIN_WIDTH,
        POPUP_MIN_HEIGHT,
        POPUP_MAX_WIDTH,
    );
    render_popup(area, buf, &popup, &lines);
}

fn render_help_popup(state: &SelectorState, area: Rect, buf: &mut Buffer) {
    if state.mode() != SelectorMode::Help {
        return;
    }

    let popup = help_popup(
        " Help ",
        HELP_POPUP_MIN_WIDTH,
        HELP_POPUP_MIN_HEIGHT,
        HELP_POPUP_MAX_WIDTH,
    );
    render_popup(area, buf, &popup, HELP_LINES);
}

fn empty_state_message(search_query: &str) -> &'static str {
    if search_query.is_empty() {
        "No readable input devices. Press Enter or Ctrl-R to refresh."
    } else {
        "No devices match the current search."
    }
}

#[cfg(test)]
mod tests {
    use super::empty_state_message;

    #[test]
    fn empty_state_message_distinguishes_search_from_empty_discovery() {
        assert_eq!(
            empty_state_message("mouse"),
            "No devices match the current search."
        );
        assert_eq!(
            empty_state_message(""),
            "No readable input devices. Press Enter or Ctrl-R to refresh."
        );
    }
}
