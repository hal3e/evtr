use std::collections::HashMap;

use super::types::InputId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputLocation {
    Absolute(usize),
    Relative(usize),
    Button(usize),
}

pub(super) struct EventIndex {
    by_event: HashMap<InputId, InputLocation>,
}

impl EventIndex {
    pub(super) fn new() -> Self {
        Self {
            by_event: HashMap::new(),
        }
    }

    pub(super) fn insert(&mut self, id: InputId, location: InputLocation) {
        self.by_event.insert(id, location);
    }

    pub(super) fn location_for(&self, id: InputId) -> Option<InputLocation> {
        self.by_event.get(&id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{EventIndex, InputLocation};
    use crate::monitor::model::InputId;

    #[test]
    fn location_for_returns_inserted_location() {
        let mut index = EventIndex::new();
        index.insert(InputId::absolute(4), InputLocation::Absolute(2));

        assert_eq!(
            index.location_for(InputId::absolute(4)),
            Some(InputLocation::Absolute(2))
        );
    }

    #[test]
    fn location_for_returns_none_for_unknown_input_id() {
        let index = EventIndex::new();

        assert_eq!(index.location_for(InputId::relative(1)), None);
    }

    #[test]
    fn insert_overwrites_existing_location_for_same_input_id() {
        let mut index = EventIndex::new();
        index.insert(InputId::key(7), InputLocation::Button(0));
        index.insert(InputId::key(7), InputLocation::Button(3));

        assert_eq!(
            index.location_for(InputId::key(7)),
            Some(InputLocation::Button(3))
        );
    }
}
