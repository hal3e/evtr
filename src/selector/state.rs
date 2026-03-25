mod filter;

use self::filter::FilterState;
use super::commands::{SelectorCommand, SelectorMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectorTransition {
    Stay,
    Exit,
    RefreshDevices,
    OpenSelection,
}

pub(super) struct SelectorState {
    filter: FilterState,
    error_message: Option<String>,
    mode: SelectorMode,
}

impl SelectorState {
    pub(super) fn new(identifiers: &[String], error_message: Option<String>) -> Self {
        Self {
            filter: FilterState::new(identifiers.len()),
            error_message,
            mode: SelectorMode::Browsing,
        }
    }

    pub(super) fn apply_discovery(
        &mut self,
        identifiers: &[String],
        error_message: Option<String>,
    ) {
        self.refresh_filter(identifiers);
        self.error_message = error_message;
    }

    pub(super) fn mode(&self) -> SelectorMode {
        self.mode
    }

    pub(super) fn reduce(
        &mut self,
        command: SelectorCommand,
        identifiers: &[String],
    ) -> SelectorTransition {
        if self.mode.is_browsing() {
            self.error_message = None;
        }

        match command {
            SelectorCommand::Exit => SelectorTransition::Exit,
            SelectorCommand::Back => self.back_transition(identifiers),
            SelectorCommand::ToggleHelp => {
                self.toggle_help();
                SelectorTransition::Stay
            }
            SelectorCommand::Refresh => SelectorTransition::RefreshDevices,
            SelectorCommand::Select => self.select_transition(identifiers.len()),
            SelectorCommand::ClearSearch => {
                self.clear_search(identifiers);
                SelectorTransition::Stay
            }
            SelectorCommand::DeleteChar => {
                self.remove_char(identifiers);
                SelectorTransition::Stay
            }
            SelectorCommand::AddChar(c) => {
                self.add_char(c, identifiers);
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

    pub(super) fn search_query(&self) -> &str {
        self.filter.search_query()
    }

    pub(super) fn filtered_indexes(&self) -> &[usize] {
        self.filter.indexes()
    }

    pub(super) fn selected_filtered_index(&self) -> usize {
        self.filter.selected_index()
    }

    pub(super) fn selected_device_index(&self) -> Option<usize> {
        self.filter.selected_item_index()
    }

    pub(super) fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    fn refresh_filter(&mut self, identifiers: &[String]) {
        self.filter.refresh(identifiers, String::as_str);
    }

    fn back_transition(&mut self, identifiers: &[String]) -> SelectorTransition {
        if self.filter.has_query() {
            self.clear_search(identifiers);
            SelectorTransition::Stay
        } else {
            SelectorTransition::Exit
        }
    }

    fn select_transition(&self, device_count: usize) -> SelectorTransition {
        if self.filter.indexes().is_empty() {
            return SelectorTransition::RefreshDevices;
        }

        if self
            .filter
            .selected_item_index()
            .is_some_and(|index| index < device_count)
        {
            SelectorTransition::OpenSelection
        } else {
            SelectorTransition::Stay
        }
    }

    fn add_char(&mut self, c: char, identifiers: &[String]) {
        self.filter.add_char(c);
        self.refresh_filter(identifiers);
    }

    fn remove_char(&mut self, identifiers: &[String]) {
        self.filter.remove_char();
        self.refresh_filter(identifiers);
    }

    fn clear_search(&mut self, identifiers: &[String]) {
        self.filter.clear_search();
        self.refresh_filter(identifiers);
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
    use super::{SelectorState, SelectorTransition};
    use crate::selector::commands::SelectorCommand;

    fn labels(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn apply_discovery_resets_selection_and_updates_empty_state_error() {
        let initial = labels(&[]);
        let mut state = SelectorState::new(&initial, None);
        for c in "mouse".chars() {
            state.add_char(c, &initial);
        }

        let refreshed = labels(&["usb mouse"]);
        state.apply_discovery(
            &refreshed,
            Some("unable to read /dev/input: read denied".to_string()),
        );

        assert_eq!(state.filtered_indexes(), &[0]);
        assert_eq!(state.selected_filtered_index(), 0);
        assert_eq!(
            state.error_message(),
            Some("unable to read /dev/input: read denied")
        );
        assert_eq!(state.search_query(), "mouse");
    }

    #[test]
    fn reduce_back_exits_only_when_search_is_empty() {
        let devices = labels(&["mouse"]);
        let mut state = SelectorState::new(&devices, None);
        state.reduce(SelectorCommand::AddChar('m'), &devices);

        assert_eq!(
            state.reduce(SelectorCommand::Back, &devices),
            SelectorTransition::Stay
        );
        assert_eq!(state.search_query(), "");

        assert_eq!(
            state.reduce(SelectorCommand::Back, &devices),
            SelectorTransition::Exit
        );
    }

    #[test]
    fn reduce_select_refreshes_when_no_results_exist() {
        let devices = labels(&[]);
        let mut state = SelectorState::new(&devices, None);

        assert_eq!(
            state.reduce(SelectorCommand::Select, &devices),
            SelectorTransition::RefreshDevices
        );
    }

    #[test]
    fn reduce_none_clears_error_while_browsing() {
        let devices = labels(&[]);
        let mut state = SelectorState::new(&devices, Some("unable to read /dev/input".to_string()));

        assert_eq!(
            state.reduce(SelectorCommand::None, &devices),
            SelectorTransition::Stay
        );
        assert_eq!(state.error_message(), None);
    }

    #[test]
    fn reduce_none_keeps_error_while_help_is_open() {
        let devices = labels(&[]);
        let mut state = SelectorState::new(&devices, Some("unable to read /dev/input".to_string()));

        assert_eq!(
            state.reduce(SelectorCommand::ToggleHelp, &devices),
            SelectorTransition::Stay
        );
        state.error_message = Some("unable to read /dev/input".to_string());

        assert_eq!(
            state.reduce(SelectorCommand::None, &devices),
            SelectorTransition::Stay
        );
        assert_eq!(state.error_message(), Some("unable to read /dev/input"));
    }
}
