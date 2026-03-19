use evdev::{AbsoluteAxisCode, AttributeSetRef, Device, EventType, InputEvent, KeyCode, PropType};

use crate::device::monitor::ComponentBootstrap;

#[derive(Clone, Debug, Default)]
struct TouchSlot {
    tracking_id: Option<i32>,
    x: Option<i32>,
    y: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TouchRange {
    Unknown,
    Fixed { min: i32, max: i32 },
    Observed { min: i32, max: i32 },
}

impl TouchRange {
    fn fixed(min: i32, max: i32) -> Self {
        Self::Fixed { min, max }
    }

    fn range(&self) -> Option<(i32, i32)> {
        match self {
            Self::Unknown => None,
            Self::Fixed { min, max } | Self::Observed { min, max } => Some((*min, *max)),
        }
    }

    fn is_known(&self) -> bool {
        self.range().is_some()
    }

    fn observe(&mut self, value: i32) {
        match self {
            Self::Unknown => {
                *self = Self::Observed {
                    min: value,
                    max: value,
                };
            }
            Self::Observed { min, max } => {
                *min = (*min).min(value);
                *max = (*max).max(value);
            }
            Self::Fixed { .. } => {}
        }
    }
}

pub(crate) struct TouchState {
    mode: TouchMode,
    current_slot: usize,
    slots: Vec<TouchSlot>,
    slot_limit: Option<usize>,
    x_range: TouchRange,
    y_range: TouchRange,
}

#[derive(Clone, Debug)]
enum TouchMode {
    None,
    MultiTouch { has_slot: bool },
    SingleTouch { contact_key: Option<KeyCode> },
}

impl TouchState {
    pub(crate) fn from_device(device: &Device) -> ComponentBootstrap<Self> {
        let Some(axes) = device.supported_absolute_axes() else {
            return ComponentBootstrap::new(Self::disabled());
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
            return ComponentBootstrap::new(Self::disabled());
        }

        let mut startup_warnings = Vec::new();
        let abs_state = match device.get_abs_state() {
            Ok(state) => Some(state),
            Err(err) => {
                startup_warnings.push(format!(
                    "unable to load touch axis state; inferring touch bounds from incoming events: {err}"
                ));
                None
            }
        };

        let mut x_range = TouchRange::Unknown;
        let mut y_range = TouchRange::Unknown;
        let mut slot_limit = None;

        let (x_axis, y_axis) = match &mode {
            TouchMode::MultiTouch { .. } => (
                AbsoluteAxisCode::ABS_MT_POSITION_X,
                AbsoluteAxisCode::ABS_MT_POSITION_Y,
            ),
            TouchMode::SingleTouch { .. } => (AbsoluteAxisCode::ABS_X, AbsoluteAxisCode::ABS_Y),
            TouchMode::None => return ComponentBootstrap::new(Self::disabled()),
        };

        if let Some(abs_state) = abs_state.as_ref() {
            if let Some(info) = abs_state.get(x_axis.0 as usize) {
                x_range = TouchRange::fixed(info.minimum, info.maximum);
            }
            if let Some(info) = abs_state.get(y_axis.0 as usize) {
                y_range = TouchRange::fixed(info.minimum, info.maximum);
            }
            if matches!(mode, TouchMode::MultiTouch { has_slot: true })
                && let Some(info) = abs_state.get(AbsoluteAxisCode::ABS_MT_SLOT.0 as usize)
            {
                slot_limit = Some(info.maximum.max(0) as usize + 1);
            }
        }

        if abs_state.is_some() && (!x_range.is_known() || !y_range.is_known()) {
            startup_warnings.push(
                "touch position range is unavailable; inferring touch bounds from incoming events"
                    .to_string(),
            );
        }

        ComponentBootstrap {
            value: Self::from_parts(mode, slot_limit, x_range, y_range),
            startup_warnings,
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
        if !self.is_touch_device() {
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
        if !self.is_touch_device() {
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
        if !self.is_touch_device() {
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
                            if self.slot_supported(slot) {
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
                        self.x_range.observe(value);
                    }
                    AbsoluteAxisCode::ABS_MT_POSITION_Y => {
                        self.ensure_slot();
                        self.slots[self.current_slot].y = Some(value);
                        self.y_range.observe(value);
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
                        self.x_range.observe(value);
                    }
                    AbsoluteAxisCode::ABS_Y => {
                        self.slots[0].y = Some(value);
                        self.y_range.observe(value);
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

    fn slot_supported(&self, slot: usize) -> bool {
        match self.slot_limit {
            Some(limit) => slot < limit,
            None => true,
        }
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

    use super::{TouchMode, TouchRange, TouchState, preferred_touch_contact_key};

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
