mod multi;
mod single;

use evdev::InputEvent;

use super::{
    TouchState,
    types::{TouchMode, TouchSlot, update_tracking_id},
};

#[derive(Clone, Copy)]
enum SlotTarget {
    Current,
    Primary,
}

#[derive(Clone, Copy)]
enum PositionAxis {
    X,
    Y,
}

impl TouchState {
    pub(crate) fn active_points(&self) -> Vec<(i32, i32)> {
        self.points_by_tracking_state(true)
    }

    pub(crate) fn inactive_points(&self) -> Vec<(i32, i32)> {
        if !self.is_touch_device() {
            return Vec::new();
        }

        self.points_by_tracking_state(false)
    }

    pub(crate) fn update(&mut self, event: &InputEvent) {
        if !self.is_touch_device() {
            return;
        }

        match self.mode {
            TouchMode::MultiTouch { .. } => multi::apply(self, event),
            TouchMode::SingleTouch { contact_key } => single::apply(self, contact_key, event),
            TouchMode::None => {}
        }
    }

    fn points_by_tracking_state(&self, active: bool) -> Vec<(i32, i32)> {
        if !self.is_touch_device() {
            return Vec::new();
        }

        self.slots
            .iter()
            .filter_map(|slot| match (slot.tracking_id.is_some(), slot.x, slot.y) {
                (tracking, Some(x), Some(y)) if tracking == active => Some((x, y)),
                _ => None,
            })
            .collect()
    }

    fn select_slot_value(&mut self, value: i32) {
        if value < 0 {
            return;
        }

        let slot = value as usize;
        if self.slot_supported(slot) {
            self.current_slot = slot;
            self.ensure_slot_capacity();
        }
    }

    fn slot_supported(&self, slot: usize) -> bool {
        match self.slot_limit {
            Some(limit) => slot < limit,
            None => true,
        }
    }

    fn slot_mut(&mut self, target: SlotTarget) -> Option<&mut TouchSlot> {
        match target {
            SlotTarget::Current => {
                self.ensure_slot_capacity();
                self.slots.get_mut(self.current_slot)
            }
            SlotTarget::Primary => self.slots.first_mut(),
        }
    }

    fn set_slot_tracking_id(
        &mut self,
        target: SlotTarget,
        tracking_id: Option<i32>,
        clear_position: bool,
    ) {
        if let Some(slot) = self.slot_mut(target) {
            update_tracking_id(slot, tracking_id, clear_position);
        }
    }

    fn arm_primary_slot_without_reset(&mut self) {
        self.set_slot_tracking_id(SlotTarget::Primary, Some(0), false);
    }

    fn update_slot_position(&mut self, target: SlotTarget, axis: PositionAxis, value: i32) {
        if let Some(slot) = self.slot_mut(target) {
            match axis {
                PositionAxis::X => slot.x = Some(value),
                PositionAxis::Y => slot.y = Some(value),
            }
        }
        self.observe_range(axis, value);
    }

    fn observe_range(&mut self, axis: PositionAxis, value: i32) {
        match axis {
            PositionAxis::X => self.x_range.observe(value),
            PositionAxis::Y => self.y_range.observe(value),
        }
    }

    fn ensure_slot_capacity(&mut self) {
        let target = match self.slot_limit {
            Some(0) => return,
            Some(limit) => {
                if self.current_slot >= limit {
                    self.current_slot = limit - 1;
                }
                (self.current_slot + 1).min(limit)
            }
            None => self.current_slot + 1,
        };

        if target > self.slots.len() {
            self.slots.resize(target, TouchSlot::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use evdev::{AbsoluteAxisCode, EventType, InputEvent, KeyCode};

    use super::super::{
        TouchState,
        types::{TouchMode, TouchRange},
    };

    fn abs(axis: AbsoluteAxisCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::ABSOLUTE.0, axis.0, value)
    }

    fn key(code: KeyCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::KEY.0, code.0, value)
    }

    fn single_touch_state(contact_key: Option<KeyCode>) -> TouchState {
        TouchState::from_parts(
            TouchMode::SingleTouch { contact_key },
            None,
            TouchRange::fixed(0, 100),
            TouchRange::fixed(0, 100),
        )
    }

    fn pending_single_touch_state(contact_key: Option<KeyCode>) -> TouchState {
        TouchState::from_parts(
            TouchMode::SingleTouch { contact_key },
            None,
            TouchRange::Unknown,
            TouchRange::Unknown,
        )
    }

    fn multi_touch_state() -> TouchState {
        TouchState::from_parts(
            TouchMode::MultiTouch { has_slot: true },
            Some(2),
            TouchRange::fixed(0, 100),
            TouchRange::fixed(0, 100),
        )
    }

    fn pending_multi_touch_state() -> TouchState {
        TouchState::from_parts(
            TouchMode::MultiTouch { has_slot: true },
            None,
            TouchRange::Unknown,
            TouchRange::Unknown,
        )
    }

    #[test]
    fn single_touch_with_contact_key_moves_between_active_and_inactive_points() {
        let mut state = single_touch_state(Some(KeyCode::BTN_TOUCH));

        state.update(&key(KeyCode::BTN_TOUCH, 1));
        state.update(&abs(AbsoluteAxisCode::ABS_X, 25));
        state.update(&abs(AbsoluteAxisCode::ABS_Y, 75));

        assert_eq!(state.active_points(), vec![(25, 75)]);
        assert!(state.inactive_points().is_empty());

        state.update(&key(KeyCode::BTN_TOUCH, 0));

        assert!(state.active_points().is_empty());
        assert_eq!(state.inactive_points(), vec![(25, 75)]);
    }

    #[test]
    fn single_touch_without_contact_key_arms_on_position_updates() {
        let mut state = single_touch_state(None);

        state.update(&abs(AbsoluteAxisCode::ABS_X, 10));
        state.update(&abs(AbsoluteAxisCode::ABS_Y, 20));

        assert_eq!(state.active_points(), vec![(10, 20)]);
        assert!(state.inactive_points().is_empty());
    }

    #[test]
    fn pending_touch_ranges_become_renderable_after_events_arrive() {
        let mut state = pending_single_touch_state(None);

        assert!(state.is_touch_device());
        assert!(!state.enabled());

        state.update(&abs(AbsoluteAxisCode::ABS_X, 10));
        assert!(!state.enabled());
        assert_eq!(state.x_range(), Some((10, 10)));
        assert_eq!(state.y_range(), None);

        state.update(&abs(AbsoluteAxisCode::ABS_Y, 20));
        state.update(&abs(AbsoluteAxisCode::ABS_X, 5));
        state.update(&abs(AbsoluteAxisCode::ABS_Y, 25));

        assert!(state.enabled());
        assert_eq!(state.x_range(), Some((5, 10)));
        assert_eq!(state.y_range(), Some((20, 25)));
        assert_eq!(state.active_points(), vec![(5, 25)]);
    }

    #[test]
    fn pending_multi_touch_state_grows_slots_from_events() {
        let mut state = pending_multi_touch_state();

        state.update(&abs(AbsoluteAxisCode::ABS_MT_SLOT, 2));
        state.update(&abs(AbsoluteAxisCode::ABS_MT_TRACKING_ID, 42));
        state.update(&abs(AbsoluteAxisCode::ABS_MT_POSITION_X, 40));
        state.update(&abs(AbsoluteAxisCode::ABS_MT_POSITION_Y, 60));

        assert_eq!(state.slots.len(), 3);
        assert!(state.enabled());
        assert_eq!(state.x_range(), Some((40, 40)));
        assert_eq!(state.y_range(), Some((60, 60)));
        assert_eq!(state.active_points(), vec![(40, 60)]);
    }

    #[test]
    fn multi_touch_slot_selection_stays_within_declared_limit() {
        let mut state = multi_touch_state();

        state.update(&abs(AbsoluteAxisCode::ABS_MT_SLOT, 1));
        assert_eq!(state.current_slot, 1);

        state.update(&abs(AbsoluteAxisCode::ABS_MT_SLOT, 5));

        assert_eq!(state.current_slot, 1);
        assert_eq!(state.slots.len(), 2);
    }

    #[test]
    fn multi_touch_release_keeps_last_position_as_inactive_point() {
        let mut state = multi_touch_state();

        state.update(&abs(AbsoluteAxisCode::ABS_MT_SLOT, 1));
        state.update(&abs(AbsoluteAxisCode::ABS_MT_TRACKING_ID, 42));
        state.update(&abs(AbsoluteAxisCode::ABS_MT_POSITION_X, 40));
        state.update(&abs(AbsoluteAxisCode::ABS_MT_POSITION_Y, 60));

        assert_eq!(state.active_points(), vec![(40, 60)]);

        state.update(&abs(AbsoluteAxisCode::ABS_MT_TRACKING_ID, -1));

        assert!(state.active_points().is_empty());
        assert_eq!(state.inactive_points(), vec![(40, 60)]);
    }
}
