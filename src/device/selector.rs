mod commands;
mod discovery;
mod view;

use std::{io, path::Path};

use crossterm::event::{Event, EventStream as TermEventStream, KeyEvent, KeyEventKind};
use evdev::Device;
use futures::StreamExt;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::DefaultTerminal;

use self::{
    commands::{SelectionAction, SelectorCommand, SelectorMode, command_for},
    discovery::{DiscoveryResult, discover_devices},
    view::render_selector,
};
use super::State;
use crate::error::{Error, ErrorArea, Result};

const PAGE_SCROLL_SIZE: usize = 10;

#[derive(Debug)]
pub struct DeviceInfo {
    pub device: Device,
    pub identifier: String,
}

fn filtered_indexes_by_query<T, F>(
    items: &[T],
    query: &str,
    matcher: &SkimMatcherV2,
    identifier_of: F,
) -> Vec<usize>
where
    F: Fn(&T) -> &str,
{
    if query.is_empty() {
        return (0..items.len()).collect();
    }

    let mut scored_items: Vec<(usize, i64)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matcher
                .fuzzy_match(identifier_of(item), query)
                .map(|score| (index, score))
        })
        .collect();

    scored_items.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    scored_items.into_iter().map(|(index, _)| index).collect()
}

pub struct DeviceSelector {
    devices: Vec<DeviceInfo>,
    filtered_indexes: Vec<usize>,
    selected_filtered_index: usize,
    search_query: String,
    matcher: SkimMatcherV2,
    error_message: Option<String>,
    mode: SelectorMode,
}

impl DeviceSelector {
    fn new(error_message: Option<String>) -> Self {
        let discovery = discover_devices(Self::open_device);
        let discovery_message = discovery.error_message();
        let devices = discovery.devices;
        let filtered_indexes = (0..devices.len()).collect();

        Self {
            devices,
            filtered_indexes,
            selected_filtered_index: 0,
            search_query: String::new(),
            matcher: SkimMatcherV2::default(),
            error_message: error_message.or(discovery_message),
            mode: SelectorMode::Browsing,
        }
    }

    fn open_device(path: &Path) -> io::Result<DeviceInfo> {
        let device = Device::open(path)?;
        let name = device.name().unwrap_or("Unknown Device").to_string();
        let path = path.to_string_lossy().to_string();

        Ok(DeviceInfo {
            device,
            identifier: format!("{name} ({path})"),
        })
    }

    fn apply_discovery(&mut self, discovery: DiscoveryResult<DeviceInfo>) {
        let error_message = discovery.error_message();
        self.devices = discovery.devices;
        self.update_filter();
        self.error_message = error_message;
    }

    fn refresh_devices(&mut self) {
        self.apply_discovery(discover_devices(Self::open_device));
    }

    pub async fn run(
        terminal: &mut DefaultTerminal,
        error_message: Option<String>,
    ) -> Result<State> {
        let mut selector = Self::new(error_message);
        let mut term_events = TermEventStream::new();

        loop {
            terminal
                .draw(|frame| {
                    render_selector(&mut selector, frame.area(), frame.buffer_mut());
                })
                .map_err(|err| Error::io(ErrorArea::Selector, "selector draw", err))?;

            match term_events.next().await {
                Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                    if let Some(state) = selector.handle_key_press(key) {
                        return Ok(state);
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(err)) => {
                    return Err(Error::io(ErrorArea::Selector, "terminal event stream", err));
                }
                None => {
                    return Err(Error::stream_ended(
                        ErrorArea::Selector,
                        "terminal event stream",
                    ));
                }
            }
        }
    }

    fn update_filter(&mut self) {
        self.filtered_indexes =
            filtered_indexes_by_query(&self.devices, &self.search_query, &self.matcher, |device| {
                device.identifier.as_str()
            });
        self.selected_filtered_index = 0;
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> Option<State> {
        let mode = self.mode;
        if mode == SelectorMode::Browsing {
            self.error_message = None;
        }
        self.apply_command(command_for(key, mode))
    }

    fn apply_command(&mut self, command: SelectorCommand) -> Option<State> {
        match command {
            SelectorCommand::Exit => Some(State::Exit),
            SelectorCommand::Back => self.back(),
            SelectorCommand::ToggleHelp => {
                self.toggle_help();
                None
            }
            SelectorCommand::Refresh => {
                self.refresh_devices();
                None
            }
            SelectorCommand::Select => self.select_or_refresh(),
            SelectorCommand::ClearSearch => {
                self.clear_search();
                None
            }
            SelectorCommand::DeleteChar => {
                self.remove_char();
                None
            }
            SelectorCommand::AddChar(c) => {
                self.add_char(c);
                None
            }
            SelectorCommand::MoveUp => {
                self.move_selection_by(-1);
                None
            }
            SelectorCommand::MoveDown => {
                self.move_selection_by(1);
                None
            }
            SelectorCommand::PageUp => {
                self.move_selection_by(-(PAGE_SCROLL_SIZE as i32));
                None
            }
            SelectorCommand::PageDown => {
                self.move_selection_by(PAGE_SCROLL_SIZE as i32);
                None
            }
            SelectorCommand::Home => {
                self.select_index(0);
                None
            }
            SelectorCommand::End => {
                if let Some(last_index) = self.filtered_indexes.len().checked_sub(1) {
                    self.select_index(last_index);
                }
                None
            }
            SelectorCommand::None => None,
        }
    }

    fn back(&mut self) -> Option<State> {
        if self.search_query.is_empty() {
            Some(State::Exit)
        } else {
            self.clear_search();
            None
        }
    }

    fn move_selection_by(&mut self, delta: i32) {
        let len = self.filtered_indexes.len();
        if len == 0 || delta == 0 {
            return;
        }

        let max_index = len - 1;
        let target = self.selected_filtered_index as i32 + delta;
        self.selected_filtered_index = target.clamp(0, max_index as i32) as usize;
    }

    fn select_index(&mut self, index: usize) {
        if let Some(max_index) = self.filtered_indexes.len().checked_sub(1) {
            self.selected_filtered_index = index.min(max_index);
        }
    }

    fn add_char(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filter();
    }

    fn remove_char(&mut self) {
        self.search_query.pop();
        self.update_filter();
    }

    fn clear_search(&mut self) {
        self.search_query.clear();
        self.update_filter();
    }

    fn selection_action(&self) -> Option<SelectionAction> {
        if self.filtered_indexes.is_empty() {
            return Some(SelectionAction::Refresh);
        }

        self.filtered_indexes
            .get(self.selected_filtered_index)
            .copied()
            .map(SelectionAction::Open)
    }

    fn select_or_refresh(&mut self) -> Option<State> {
        match self.selection_action() {
            Some(SelectionAction::Refresh) => {
                self.refresh_devices();
                None
            }
            Some(SelectionAction::Open(index)) => {
                Some(State::Monitor(Box::new(self.devices.swap_remove(index))))
            }
            None => None,
        }
    }

    fn toggle_help(&mut self) {
        self.mode = match self.mode {
            SelectorMode::Browsing => SelectorMode::Help,
            SelectorMode::Help => SelectorMode::Browsing,
        };
    }
}

#[cfg(test)]
mod tests {
    use fuzzy_matcher::skim::SkimMatcherV2;

    use super::{DeviceSelector, SelectionAction, SelectorMode, State, filtered_indexes_by_query};

    #[test]
    fn filtered_indexes_by_query_returns_all_items_for_empty_query() {
        let matcher = SkimMatcherV2::default();
        let identifiers = vec!["usb mouse", "gamepad"];

        let indexes = filtered_indexes_by_query(&identifiers, "", &matcher, |item| item);

        assert_eq!(indexes, vec![0, 1]);
    }

    #[test]
    fn filtered_indexes_by_query_returns_empty_when_nothing_matches() {
        let matcher = SkimMatcherV2::default();
        let identifiers = vec!["usb mouse", "gamepad"];

        let indexes = filtered_indexes_by_query(&identifiers, "keyboard", &matcher, |item| item);

        assert!(indexes.is_empty());
    }

    #[test]
    fn apply_discovery_resets_selection_and_updates_empty_state_error() {
        let mut selector = DeviceSelector {
            devices: Vec::new(),
            filtered_indexes: vec![0, 1],
            selected_filtered_index: 1,
            search_query: "mouse".to_string(),
            matcher: SkimMatcherV2::default(),
            error_message: None,
            mode: SelectorMode::Browsing,
        };

        let discovery = crate::device::selector::discovery::DiscoveryResult::read_dir_failed(
            "/dev/input",
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "read denied"),
        );

        selector.apply_discovery(discovery);

        assert!(selector.devices.is_empty());
        assert!(selector.filtered_indexes.is_empty());
        assert_eq!(selector.selected_filtered_index, 0);
        assert_eq!(
            selector.error_message,
            Some("unable to read /dev/input: read denied".to_string())
        );
        assert_eq!(selector.search_query, "mouse");
    }

    fn selector_with(filtered_indexes: Vec<usize>, search_query: &str) -> DeviceSelector {
        DeviceSelector {
            devices: Vec::new(),
            filtered_indexes,
            selected_filtered_index: 0,
            search_query: search_query.to_string(),
            matcher: SkimMatcherV2::default(),
            error_message: None,
            mode: SelectorMode::Browsing,
        }
    }

    #[test]
    fn back_exits_only_when_search_is_empty() {
        let mut selector = selector_with(vec![0], "mouse");

        assert!(selector.back().is_none());
        assert!(selector.search_query.is_empty());

        assert!(matches!(selector.back(), Some(State::Exit)));
    }

    #[test]
    fn selection_action_refreshes_when_no_results_exist() {
        let selector = selector_with(Vec::new(), "");

        assert_eq!(selector.selection_action(), Some(SelectionAction::Refresh));
    }

    #[test]
    fn selection_action_uses_selected_filtered_index() {
        let mut selector = selector_with(vec![2, 5, 7], "");
        selector.selected_filtered_index = 1;

        assert_eq!(selector.selection_action(), Some(SelectionAction::Open(5)));
    }
}
