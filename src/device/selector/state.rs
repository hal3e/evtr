mod filter;

use self::filter::FilterState;
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
    filter: FilterState,
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
            filter: FilterState::new(devices.len()),
            devices,
            error_message: error_message.or(discovery_message),
            mode: SelectorMode::Browsing,
        }
    }

    pub(crate) fn apply_discovery(&mut self, discovery: DiscoveryResult<DeviceInfo>) {
        let error_message = discovery.error_message();
        self.devices = discovery.devices;
        self.refresh_filter();
        self.error_message = error_message;
    }

    pub(crate) fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    pub(crate) fn mode(&self) -> SelectorMode {
        self.mode
    }

    pub(crate) fn back_action(&self) -> BackAction {
        if self.filter.has_query() {
            BackAction::ClearSearch
        } else {
            BackAction::Exit
        }
    }

    pub(crate) fn move_selection_by(&mut self, delta: i32) {
        self.filter.move_selection_by(delta);
    }

    pub(crate) fn select_first(&mut self) {
        self.filter.select_first();
    }

    pub(crate) fn select_last(&mut self) {
        self.filter.select_last();
    }

    pub(crate) fn add_char(&mut self, c: char) {
        self.filter.add_char(c);
        self.refresh_filter();
    }

    pub(crate) fn remove_char(&mut self) {
        self.filter.remove_char();
        self.refresh_filter();
    }

    pub(crate) fn clear_search(&mut self) {
        self.filter.clear_search();
        self.refresh_filter();
    }

    pub(crate) fn toggle_help(&mut self) {
        self.mode = match self.mode {
            SelectorMode::Browsing => SelectorMode::Help,
            SelectorMode::Help => SelectorMode::Browsing,
        };
    }

    pub(crate) fn selection_action(&self) -> Option<SelectionAction> {
        if self.filter.indexes().is_empty() {
            return Some(SelectionAction::Refresh);
        }

        self.filter
            .selected_item_index()
            .filter(|index| *index < self.devices.len())
            .map(|_| SelectionAction::OpenSelected)
    }

    pub(crate) fn take_selected_device(&mut self) -> Option<DeviceInfo> {
        let index = self.filter.selected_item_index()?;
        if index >= self.devices.len() {
            return None;
        }

        Some(self.devices.swap_remove(index))
    }

    pub(crate) fn search_query(&self) -> &str {
        self.filter.search_query()
    }

    pub(crate) fn filtered_indexes(&self) -> &[usize] {
        self.filter.indexes()
    }

    pub(crate) fn selected_filtered_index(&self) -> usize {
        self.filter.selected_index()
    }

    pub(crate) fn device_identifier(&self, index: usize) -> Option<&str> {
        self.devices
            .get(index)
            .map(|device| device.identifier.as_str())
    }

    pub(crate) fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    fn refresh_filter(&mut self) {
        self.filter.refresh(&self.devices, device_identifier);
    }
}

fn device_identifier(device: &DeviceInfo) -> &str {
    device.identifier.as_str()
}

#[cfg(test)]
mod tests {
    use super::{BackAction, SelectionAction, SelectorState};
    use crate::device::selector::discovery::DiscoveryResult;

    #[test]
    fn apply_discovery_resets_selection_and_updates_empty_state_error() {
        let mut state = SelectorState::new(DiscoveryResult::new(), None);
        for c in "mouse".chars() {
            state.add_char(c);
        }

        let discovery = crate::device::selector::discovery::DiscoveryResult::read_dir_failed(
            "/dev/input",
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "read denied"),
        );

        state.apply_discovery(discovery);

        assert!(state.filtered_indexes().is_empty());
        assert_eq!(state.selected_filtered_index(), 0);
        assert_eq!(
            state.error_message(),
            Some("unable to read /dev/input: read denied")
        );
        assert_eq!(state.search_query(), "mouse");
    }

    #[test]
    fn back_action_exits_only_when_search_is_empty() {
        let mut state = SelectorState::new(DiscoveryResult::new(), None);
        for c in "mouse".chars() {
            state.add_char(c);
        }

        assert_eq!(state.back_action(), BackAction::ClearSearch);

        state.clear_search();

        assert_eq!(state.back_action(), BackAction::Exit);
    }

    #[test]
    fn selection_action_refreshes_when_no_results_exist() {
        let state = SelectorState::new(DiscoveryResult::new(), None);

        assert_eq!(state.selection_action(), Some(SelectionAction::Refresh));
    }
}
