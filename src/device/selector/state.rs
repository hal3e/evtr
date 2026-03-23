use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

use super::{DeviceInfo, commands::SelectorMode, discovery::DiscoveryResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackAction {
    Exit,
    ClearSearch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectionAction {
    Refresh,
    OpenSelected,
}

pub(crate) struct SelectorState {
    devices: Vec<DeviceInfo>,
    filtered_indexes: Vec<usize>,
    selected_filtered_index: usize,
    search_query: String,
    matcher: SkimMatcherV2,
    error_message: Option<String>,
    mode: SelectorMode,
}

impl SelectorState {
    pub(crate) fn new(
        discovery: DiscoveryResult<DeviceInfo>,
        error_message: Option<String>,
    ) -> Self {
        let discovery_message = discovery.error_message();
        let devices = discovery.devices;

        Self {
            filtered_indexes: (0..devices.len()).collect(),
            devices,
            selected_filtered_index: 0,
            search_query: String::new(),
            matcher: SkimMatcherV2::default(),
            error_message: error_message.or(discovery_message),
            mode: SelectorMode::Browsing,
        }
    }

    pub(crate) fn apply_discovery(&mut self, discovery: DiscoveryResult<DeviceInfo>) {
        let error_message = discovery.error_message();
        self.devices = discovery.devices;
        self.update_filter();
        self.error_message = error_message;
    }

    pub(crate) fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    pub(crate) fn mode(&self) -> SelectorMode {
        self.mode
    }

    pub(crate) fn back_action(&self) -> BackAction {
        if self.search_query.is_empty() {
            BackAction::Exit
        } else {
            BackAction::ClearSearch
        }
    }

    pub(crate) fn move_selection_by(&mut self, delta: i32) {
        let len = self.filtered_indexes.len();
        if len == 0 || delta == 0 {
            return;
        }

        let max_index = len - 1;
        let target = self.selected_filtered_index as i32 + delta;
        self.selected_filtered_index = target.clamp(0, max_index as i32) as usize;
    }

    pub(crate) fn select_index(&mut self, index: usize) {
        if let Some(max_index) = self.filtered_indexes.len().checked_sub(1) {
            self.selected_filtered_index = index.min(max_index);
        }
    }

    pub(crate) fn add_char(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filter();
    }

    pub(crate) fn remove_char(&mut self) {
        self.search_query.pop();
        self.update_filter();
    }

    pub(crate) fn clear_search(&mut self) {
        self.search_query.clear();
        self.update_filter();
    }

    pub(crate) fn toggle_help(&mut self) {
        self.mode = match self.mode {
            SelectorMode::Browsing => SelectorMode::Help,
            SelectorMode::Help => SelectorMode::Browsing,
        };
    }

    pub(crate) fn selection_action(&self) -> Option<SelectionAction> {
        if self.filtered_indexes.is_empty() {
            return Some(SelectionAction::Refresh);
        }

        selected_item_index(&self.filtered_indexes, self.selected_filtered_index)
            .filter(|index| *index < self.devices.len())
            .map(|_| SelectionAction::OpenSelected)
    }

    pub(crate) fn take_selected_device(&mut self) -> Option<DeviceInfo> {
        let index = selected_item_index(&self.filtered_indexes, self.selected_filtered_index)?;
        if index >= self.devices.len() {
            return None;
        }

        Some(self.devices.swap_remove(index))
    }

    pub(crate) fn search_query(&self) -> &str {
        &self.search_query
    }

    pub(crate) fn filtered_indexes(&self) -> &[usize] {
        &self.filtered_indexes
    }

    pub(crate) fn selected_filtered_index(&self) -> usize {
        self.selected_filtered_index
    }

    pub(crate) fn device_identifier(&self, index: usize) -> Option<&str> {
        self.devices
            .get(index)
            .map(|device| device.identifier.as_str())
    }

    pub(crate) fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    fn update_filter(&mut self) {
        self.filtered_indexes =
            filtered_indexes_by_query(&self.devices, &self.search_query, &self.matcher, |device| {
                device.identifier.as_str()
            });
        self.selected_filtered_index = 0;
    }
}

fn selected_item_index(
    filtered_indexes: &[usize],
    selected_filtered_index: usize,
) -> Option<usize> {
    filtered_indexes.get(selected_filtered_index).copied()
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

#[cfg(test)]
mod tests {
    use fuzzy_matcher::skim::SkimMatcherV2;

    use super::{
        BackAction, SelectionAction, SelectorState, filtered_indexes_by_query, selected_item_index,
    };
    use crate::device::selector::commands::SelectorMode;

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
        let mut state = selector_with(vec![0, 1], "mouse");

        let discovery = crate::device::selector::discovery::DiscoveryResult::read_dir_failed(
            "/dev/input",
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "read denied"),
        );

        state.apply_discovery(discovery);

        assert!(state.filtered_indexes.is_empty());
        assert_eq!(state.selected_filtered_index, 0);
        assert_eq!(
            state.error_message,
            Some("unable to read /dev/input: read denied".to_string())
        );
        assert_eq!(state.search_query, "mouse");
    }

    #[test]
    fn back_action_exits_only_when_search_is_empty() {
        let mut state = selector_with(vec![0], "mouse");

        assert_eq!(state.back_action(), BackAction::ClearSearch);

        state.clear_search();

        assert_eq!(state.back_action(), BackAction::Exit);
    }

    #[test]
    fn selection_action_refreshes_when_no_results_exist() {
        let state = selector_with(Vec::new(), "");

        assert_eq!(state.selection_action(), Some(SelectionAction::Refresh));
    }

    #[test]
    fn selected_item_index_uses_selected_filtered_index() {
        assert_eq!(selected_item_index(&[2, 5, 7], 1), Some(5));
        assert_eq!(selected_item_index(&[2, 5, 7], 4), None);
    }

    fn selector_with(filtered_indexes: Vec<usize>, search_query: &str) -> SelectorState {
        SelectorState {
            devices: Vec::new(),
            filtered_indexes,
            selected_filtered_index: 0,
            search_query: search_query.to_string(),
            matcher: SkimMatcherV2::default(),
            error_message: None,
            mode: SelectorMode::Browsing,
        }
    }
}
