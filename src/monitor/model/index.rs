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
