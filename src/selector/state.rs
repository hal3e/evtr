mod filter;

use self::filter::FilterState;
use super::{
    DeviceInfo,
    commands::{SelectorCommand, SelectorMode},
    discovery::DiscoveryResult,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorTransition {
    Stay,
    Exit,
    RefreshDevices,
    OpenSelection,
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

    pub(crate) fn mode(&self) -> SelectorMode {
        self.mode
    }

    pub(crate) fn reduce(&mut self, command: SelectorCommand) -> SelectorTransition {
        if self.mode.is_browsing() {
            self.error_message = None;
        }

        match command {
            SelectorCommand::Exit => SelectorTransition::Exit,
            SelectorCommand::Back => self.back_transition(),
            SelectorCommand::ToggleHelp => {
                self.toggle_help();
                SelectorTransition::Stay
            }
            SelectorCommand::Refresh => SelectorTransition::RefreshDevices,
            SelectorCommand::Select => self.select_transition(),
            SelectorCommand::ClearSearch => {
                self.clear_search();
                SelectorTransition::Stay
            }
            SelectorCommand::DeleteChar => {
                self.remove_char();
                SelectorTransition::Stay
            }
            SelectorCommand::AddChar(c) => {
                self.add_char(c);
                SelectorTransition::Stay
            }
            SelectorCommand::MoveUp => {
                self.filter.move_up();
                SelectorTransition::Stay
            }
            SelectorCommand::MoveDown => {
                self.filter.move_down();
                SelectorTransition::Stay
            }
            SelectorCommand::PageUp => {
                self.filter.page_up();
                SelectorTransition::Stay
            }
            SelectorCommand::PageDown => {
                self.filter.page_down();
                SelectorTransition::Stay
            }
            SelectorCommand::Home => {
                self.filter.home();
                SelectorTransition::Stay
            }
            SelectorCommand::End => {
                self.filter.end();
                SelectorTransition::Stay
            }
            SelectorCommand::None => SelectorTransition::Stay,
        }
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

    fn back_transition(&mut self) -> SelectorTransition {
        if self.filter.has_query() {
            self.clear_search();
            SelectorTransition::Stay
        } else {
            SelectorTransition::Exit
        }
    }

    fn select_transition(&self) -> SelectorTransition {
        if self.filter.indexes().is_empty() {
            return SelectorTransition::RefreshDevices;
        }

        if self
            .filter
            .selected_item_index()
            .is_some_and(|index| index < self.devices.len())
        {
            SelectorTransition::OpenSelection
        } else {
            SelectorTransition::Stay
        }
    }

    fn add_char(&mut self, c: char) {
        self.filter.add_char(c);
        self.refresh_filter();
    }

    fn remove_char(&mut self) {
        self.filter.remove_char();
        self.refresh_filter();
    }

    fn clear_search(&mut self) {
        self.filter.clear_search();
        self.refresh_filter();
    }

    fn toggle_help(&mut self) {
        self.mode = match self.mode {
            SelectorMode::Browsing => SelectorMode::Help,
            SelectorMode::Help => SelectorMode::Browsing,
        };
    }
}

fn device_identifier(device: &DeviceInfo) -> &str {
    device.identifier.as_str()
}

#[cfg(test)]
mod tests {
    use super::{SelectorState, SelectorTransition};
    use crate::selector::commands::SelectorCommand;
    use crate::selector::discovery::DiscoveryResult;

    #[test]
    fn apply_discovery_resets_selection_and_updates_empty_state_error() {
        let mut state = SelectorState::new(DiscoveryResult::new(), None);
        for c in "mouse".chars() {
            state.add_char(c);
        }

        let discovery = crate::selector::discovery::DiscoveryResult::read_dir_failed(
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
    fn reduce_back_exits_only_when_search_is_empty() {
        let mut state = SelectorState::new(DiscoveryResult::new(), None);
        state.reduce(SelectorCommand::AddChar('m'));

        assert_eq!(
            state.reduce(SelectorCommand::Back),
            SelectorTransition::Stay
        );
        assert_eq!(state.search_query(), "");

        assert_eq!(
            state.reduce(SelectorCommand::Back),
            SelectorTransition::Exit
        );
    }

    #[test]
    fn reduce_select_refreshes_when_no_results_exist() {
        let mut state = SelectorState::new(DiscoveryResult::new(), None);

        assert_eq!(
            state.reduce(SelectorCommand::Select),
            SelectorTransition::RefreshDevices
        );
    }

    #[test]
    fn reduce_none_clears_error_while_browsing() {
        let mut state = SelectorState::new(
            DiscoveryResult::new(),
            Some("unable to read /dev/input".to_string()),
        );

        assert_eq!(
            state.reduce(SelectorCommand::None),
            SelectorTransition::Stay
        );
        assert_eq!(state.error_message(), None);
    }

    #[test]
    fn reduce_none_keeps_error_while_help_is_open() {
        let mut state = SelectorState::new(
            DiscoveryResult::new(),
            Some("unable to read /dev/input".to_string()),
        );

        assert_eq!(
            state.reduce(SelectorCommand::ToggleHelp),
            SelectorTransition::Stay
        );
        state.error_message = Some("unable to read /dev/input".to_string());

        assert_eq!(
            state.reduce(SelectorCommand::None),
            SelectorTransition::Stay
        );
        assert_eq!(state.error_message(), Some("unable to read /dev/input"));
    }
}
