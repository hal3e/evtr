use evdev::{AbsoluteAxisCode, AttributeSetRef, Device, EventType, InputEvent, KeyCode, PropType};

use crate::device::monitor::InitialStateLoad;

#[derive(Clone, Debug, Default)]
struct TouchSlot {
    tracking_id: Option<i32>,
    x: Option<i32>,
    y: Option<i32>,
}

pub(crate) struct TouchState {
    mode: TouchMode,
    current_slot: usize,
    slots: Vec<TouchSlot>,
    max_slots: usize,
    x_range: (i32, i32),
    y_range: (i32, i32),
}

#[derive(Clone, Debug)]
enum TouchMode {
    None,
    MultiTouch { has_slot: bool },
    SingleTouch { contact_key: Option<KeyCode> },
}

impl TouchState {
    pub(crate) fn from_device(device: &Device) -> (Self, InitialStateLoad) {
        let Some(axes) = device.supported_absolute_axes() else {
            return (Self::disabled(), InitialStateLoad::Full);
        };

        let supports_mt_x = axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_X);
        let supports_mt_y = axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_Y);
        let supports_slot = axes.contains(AbsoluteAxisCode::ABS_MT_SLOT);
        let supports_abs_x = axes.contains(AbsoluteAxisCode::ABS_X);
        let supports_abs_y = axes.contains(AbsoluteAxisCode::ABS_Y);

        let properties = device.properties();
        let has_touch_props = properties.contains(PropType::DIRECT)
            || properties.contains(PropType::BUTTONPAD)
            || properties.contains(PropType::SEMI_MT)
            || properties.contains(PropType::TOPBUTTONPAD);
        let touch_contact_key = device
            .supported_keys()
            .and_then(preferred_touch_contact_key);
        let has_touch_keys = touch_contact_key.is_some();

        let mode = if supports_mt_x && supports_mt_y {
            TouchMode::MultiTouch {
                has_slot: supports_slot,
            }
        } else if supports_abs_x && supports_abs_y && (has_touch_props || has_touch_keys) {
            TouchMode::SingleTouch {
                contact_key: touch_contact_key,
            }
        } else {
            TouchMode::None
        };

        if matches!(mode, TouchMode::None) {
            return (Self::disabled(), InitialStateLoad::Full);
        }

        let mut initial_state_load = InitialStateLoad::Full;
        let abs_state = match device.get_abs_state() {
            Ok(state) => Some(state),
            Err(err) => {
                initial_state_load.record_warning(format!(
                    "unable to load touch axis state; touch starts empty until events arrive: {err}"
                ));
                None
            }
        };
        let mut x_range = None;
        let mut y_range = None;
        let mut slot_max = None;
        let (x_axis, y_axis) = match mode {
            TouchMode::MultiTouch { .. } => (
                AbsoluteAxisCode::ABS_MT_POSITION_X,
                AbsoluteAxisCode::ABS_MT_POSITION_Y,
            ),
            TouchMode::SingleTouch { .. } => (AbsoluteAxisCode::ABS_X, AbsoluteAxisCode::ABS_Y),
            TouchMode::None => return (Self::disabled(), InitialStateLoad::Full),
        };

        if let Some(abs_state) = abs_state.as_ref() {
            if let Some(info) = abs_state.get(x_axis.0 as usize) {
                x_range = Some((info.minimum, info.maximum));
            }
            if let Some(info) = abs_state.get(y_axis.0 as usize) {
                y_range = Some((info.minimum, info.maximum));
            }
            if matches!(mode, TouchMode::MultiTouch { has_slot: true })
                && let Some(info) = abs_state.get(AbsoluteAxisCode::ABS_MT_SLOT.0 as usize)
            {
                slot_max = Some(info.maximum);
            }
        }

        let Some(x_range) = x_range else {
            initial_state_load.record_warning(
                "touch position range is unavailable; touch view stays hidden until supported state can be read",
            );
            return (Self::disabled(), initial_state_load);
        };
        let Some(y_range) = y_range else {
            initial_state_load.record_warning(
                "touch position range is unavailable; touch view stays hidden until supported state can be read",
            );
            return (Self::disabled(), initial_state_load);
        };

        let slots_len = match mode {
            TouchMode::MultiTouch { has_slot } => {
                if has_slot {
                    slot_max.map(|max| max.max(0) as usize + 1).unwrap_or(1)
                } else {
                    1
                }
            }
            TouchMode::SingleTouch { .. } => 1,
            TouchMode::None => 0,
        };

        let mut slots = vec![TouchSlot::default(); slots_len];
        if matches!(mode, TouchMode::SingleTouch { contact_key: None }) && !slots.is_empty() {
            slots[0].tracking_id = Some(0);
        }

        (
            Self {
                mode,
                current_slot: 0,
                slots,
                max_slots: slots_len,
                x_range,
                y_range,
            },
            initial_state_load,
        )
    }

    fn disabled() -> Self {
        Self {
            mode: TouchMode::None,
            current_slot: 0,
            slots: Vec::new(),
            max_slots: 0,
            x_range: (0, 1),
            y_range: (0, 1),
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        !matches!(self.mode, TouchMode::None)
    }

    pub(crate) fn is_touch_device(&self) -> bool {
        self.enabled()
    }

    pub(crate) fn x_range(&self) -> (i32, i32) {
        self.x_range
    }

    pub(crate) fn y_range(&self) -> (i32, i32) {
        self.y_range
    }

    pub(crate) fn active_points(&self) -> Vec<(i32, i32)> {
        if !self.enabled() {
            return Vec::new();
        }
        self.slots
            .iter()
            .filter_map(|slot| match (slot.tracking_id, slot.x, slot.y) {
                (Some(_), Some(x), Some(y)) => Some((x, y)),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn inactive_points(&self) -> Vec<(i32, i32)> {
        if !self.enabled() {
            return Vec::new();
        }
        self.slots
            .iter()
            .filter_map(|slot| match (slot.tracking_id, slot.x, slot.y) {
                (None, Some(x), Some(y)) => Some((x, y)),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn update(&mut self, event: &InputEvent) {
        if !self.enabled() {
            return;
        }

        match (&self.mode, event.event_type()) {
            (TouchMode::MultiTouch { .. }, EventType::ABSOLUTE) => {
                let axis = AbsoluteAxisCode(event.code());
                let value = event.value();
                match axis {
                    AbsoluteAxisCode::ABS_MT_SLOT => {
                        if value >= 0 {
                            let slot = value as usize;
                            if slot < self.max_slots {
                                self.current_slot = slot;
                                self.ensure_slot();
                            }
                        }
                    }
                    AbsoluteAxisCode::ABS_MT_TRACKING_ID => {
                        self.ensure_slot();
                        if value < 0 {
                            self.slots[self.current_slot].tracking_id = None;
                        } else {
                            self.slots[self.current_slot].tracking_id = Some(value);
                            self.slots[self.current_slot].x = None;
                            self.slots[self.current_slot].y = None;
                        }
                    }
                    AbsoluteAxisCode::ABS_MT_POSITION_X => {
                        self.ensure_slot();
                        self.slots[self.current_slot].x = Some(value);
                    }
                    AbsoluteAxisCode::ABS_MT_POSITION_Y => {
                        self.ensure_slot();
                        self.slots[self.current_slot].y = Some(value);
                    }
                    _ => {}
                }
            }
            (TouchMode::SingleTouch { contact_key }, EventType::ABSOLUTE) => {
                let axis = AbsoluteAxisCode(event.code());
                let value = event.value();
                match axis {
                    AbsoluteAxisCode::ABS_X => {
                        self.slots[0].x = Some(value);
                    }
                    AbsoluteAxisCode::ABS_Y => {
                        self.slots[0].y = Some(value);
                    }
                    _ => {}
                }
                if contact_key.is_none() {
                    self.slots[0].tracking_id = Some(0);
                }
            }
            (
                TouchMode::SingleTouch {
                    contact_key: Some(_),
                },
                EventType::KEY,
            ) => {
                let key = KeyCode(event.code());
                if matches!(key, KeyCode::BTN_TOUCH | KeyCode::BTN_TOOL_FINGER) {
                    if event.value() == 0 {
                        self.slots[0].tracking_id = None;
                    } else {
                        self.slots[0].tracking_id = Some(0);
                        self.slots[0].x = None;
                        self.slots[0].y = None;
                    }
                }
            }
            _ => {}
        }
    }

    fn ensure_slot(&mut self) {
        if self.max_slots == 0 {
            return;
        }
        if self.current_slot >= self.max_slots {
            self.current_slot = self.max_slots - 1;
        }
        if self.current_slot >= self.slots.len() {
            let target = (self.current_slot + 1).min(self.max_slots);
            if target > self.slots.len() {
                self.slots.resize(target, TouchSlot::default());
            }
        }
    }
}

fn preferred_touch_contact_key(keys: &AttributeSetRef<KeyCode>) -> Option<KeyCode> {
    if keys.contains(KeyCode::BTN_TOUCH) {
        Some(KeyCode::BTN_TOUCH)
    } else if keys.contains(KeyCode::BTN_TOOL_FINGER) {
        Some(KeyCode::BTN_TOOL_FINGER)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use evdev::{AbsoluteAxisCode, AttributeSet, EventType, InputEvent, KeyCode};

    use super::{TouchMode, TouchSlot, TouchState, preferred_touch_contact_key};

    fn abs(axis: AbsoluteAxisCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::ABSOLUTE.0, axis.0, value)
    }

    fn key(code: KeyCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::KEY.0, code.0, value)
    }

    fn single_touch_state(contact_key: Option<KeyCode>) -> TouchState {
        TouchState {
            mode: TouchMode::SingleTouch { contact_key },
            current_slot: 0,
            slots: vec![TouchSlot::default()],
            max_slots: 1,
            x_range: (0, 100),
            y_range: (0, 100),
        }
    }

    fn multi_touch_state() -> TouchState {
        TouchState {
            mode: TouchMode::MultiTouch { has_slot: true },
            current_slot: 0,
            slots: vec![TouchSlot::default(), TouchSlot::default()],
            max_slots: 2,
            x_range: (0, 100),
            y_range: (0, 100),
        }
    }

    #[test]
    fn preferred_touch_contact_key_prefers_btn_touch() {
        let mut keys = AttributeSet::new();
        keys.insert(KeyCode::BTN_TOOL_FINGER);
        keys.insert(KeyCode::BTN_TOUCH);

        assert_eq!(preferred_touch_contact_key(&keys), Some(KeyCode::BTN_TOUCH));
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
