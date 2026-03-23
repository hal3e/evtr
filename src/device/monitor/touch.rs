mod bootstrap;
mod types;

use evdev::{AbsoluteAxisCode, Device, EventType, InputEvent, KeyCode};

use crate::device::monitor::ComponentBootstrap;

use self::{
    bootstrap::inspect_touch_device,
    types::{TouchMode, TouchRange, TouchSlot, update_tracking_id},
};

pub(crate) struct TouchState {
    mode: TouchMode,
    current_slot: usize,
    slots: Vec<TouchSlot>,
    slot_limit: Option<usize>,
    x_range: TouchRange,
    y_range: TouchRange,
}

impl TouchState {
    pub(crate) fn from_device(device: &Device) -> ComponentBootstrap<Self> {
        let Some(bootstrap) = inspect_touch_device(device) else {
            return ComponentBootstrap::new(Self::disabled());
        };

        ComponentBootstrap {
            value: Self::from_parts(
                bootstrap.mode,
                bootstrap.slot_limit,
                bootstrap.x_range,
                bootstrap.y_range,
            ),
            startup_warnings: bootstrap.startup_warnings,
        }
    }

    fn disabled() -> Self {
        Self::from_parts(
            TouchMode::None,
            Some(0),
            TouchRange::Unknown,
            TouchRange::Unknown,
        )
    }

    fn from_parts(
        mode: TouchMode,
        slot_limit: Option<usize>,
        x_range: TouchRange,
        y_range: TouchRange,
    ) -> Self {
        let slot_limit = match &mode {
            TouchMode::None => Some(0),
            TouchMode::SingleTouch { .. } | TouchMode::MultiTouch { has_slot: false } => Some(1),
            TouchMode::MultiTouch { has_slot: true } => slot_limit,
        };

        let slots_len = match &mode {
            TouchMode::None => 0,
            _ => slot_limit.unwrap_or(1).max(1),
        };

        let mut slots = vec![TouchSlot::default(); slots_len];
        if matches!(&mode, TouchMode::SingleTouch { contact_key: None }) && !slots.is_empty() {
            slots[0].tracking_id = Some(0);
        }

        Self {
            mode,
            current_slot: 0,
            slots,
            slot_limit,
            x_range,
            y_range,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.x_range.is_known() && self.y_range.is_known()
    }

    pub(crate) fn is_touch_device(&self) -> bool {
        !matches!(self.mode, TouchMode::None)
    }

    pub(crate) fn x_range(&self) -> Option<(i32, i32)> {
        self.x_range.range()
    }

    pub(crate) fn y_range(&self) -> Option<(i32, i32)> {
        self.y_range.range()
    }

    pub(crate) fn ranges(&self) -> Option<((i32, i32), (i32, i32))> {
        Some((self.x_range()?, self.y_range()?))
    }

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

        match (self.mode, event.event_type()) {
            (TouchMode::MultiTouch { .. }, EventType::ABSOLUTE) => {
                self.handle_multi_touch_absolute(event);
            }
            (TouchMode::SingleTouch { contact_key }, EventType::ABSOLUTE) => {
                self.handle_single_touch_absolute(contact_key, event);
            }
            (
                TouchMode::SingleTouch {
                    contact_key: Some(_),
                },
                EventType::KEY,
            ) => self.handle_single_touch_key(event),
            _ => {}
        }
    }

    fn handle_multi_touch_absolute(&mut self, event: &InputEvent) {
        let axis = AbsoluteAxisCode(event.code());
        let value = event.value();

        match axis {
            AbsoluteAxisCode::ABS_MT_SLOT => self.select_slot(value),
            AbsoluteAxisCode::ABS_MT_TRACKING_ID => {
                self.set_current_slot_tracking_id((value >= 0).then_some(value));
            }
            AbsoluteAxisCode::ABS_MT_POSITION_X => self.update_current_slot_x(value),
            AbsoluteAxisCode::ABS_MT_POSITION_Y => self.update_current_slot_y(value),
            _ => {}
        }
    }

    fn handle_single_touch_absolute(&mut self, contact_key: Option<KeyCode>, event: &InputEvent) {
        let axis = AbsoluteAxisCode(event.code());
        let value = event.value();

        match axis {
            AbsoluteAxisCode::ABS_X => self.update_single_touch_x(value),
            AbsoluteAxisCode::ABS_Y => self.update_single_touch_y(value),
            _ => {}
        }

        if contact_key.is_none() {
            self.arm_single_touch_without_reset();
        }
    }

    fn handle_single_touch_key(&mut self, event: &InputEvent) {
        let key = KeyCode(event.code());
        if matches!(key, KeyCode::BTN_TOUCH | KeyCode::BTN_TOOL_FINGER) {
            self.set_single_touch_tracking_id((event.value() != 0).then_some(0), true);
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

    fn select_slot(&mut self, value: i32) {
        if value < 0 {
            return;
        }

        let slot = value as usize;
        if self.slot_supported(slot) {
            self.current_slot = slot;
            self.ensure_slot();
        }
    }

    fn slot_supported(&self, slot: usize) -> bool {
        match self.slot_limit {
            Some(limit) => slot < limit,
            None => true,
        }
    }

    fn current_slot_mut(&mut self) -> Option<&mut TouchSlot> {
        self.ensure_slot();
        self.slots.get_mut(self.current_slot)
    }

    fn single_slot_mut(&mut self) -> Option<&mut TouchSlot> {
        self.slots.first_mut()
    }

    fn set_current_slot_tracking_id(&mut self, tracking_id: Option<i32>) {
        if let Some(slot) = self.current_slot_mut() {
            update_tracking_id(slot, tracking_id, true);
        }
    }

    fn update_current_slot_x(&mut self, value: i32) {
        if let Some(slot) = self.current_slot_mut() {
            slot.x = Some(value);
        }
        self.x_range.observe(value);
    }

    fn update_current_slot_y(&mut self, value: i32) {
        if let Some(slot) = self.current_slot_mut() {
            slot.y = Some(value);
        }
        self.y_range.observe(value);
    }

    fn arm_single_touch_without_reset(&mut self) {
        if let Some(slot) = self.single_slot_mut() {
            slot.tracking_id = Some(0);
        }
    }

    fn set_single_touch_tracking_id(&mut self, tracking_id: Option<i32>, clear_position: bool) {
        if let Some(slot) = self.single_slot_mut() {
            update_tracking_id(slot, tracking_id, clear_position);
        }
    }

    fn update_single_touch_x(&mut self, value: i32) {
        if let Some(slot) = self.single_slot_mut() {
            slot.x = Some(value);
        }
        self.x_range.observe(value);
    }

    fn update_single_touch_y(&mut self, value: i32) {
        if let Some(slot) = self.single_slot_mut() {
            slot.y = Some(value);
        }
        self.y_range.observe(value);
    }

    fn ensure_slot(&mut self) {
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

    use super::{
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
