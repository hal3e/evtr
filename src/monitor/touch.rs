mod bootstrap;
mod types;
mod update;

use evdev::Device;

use self::{
    super::bootstrap::Bootstrapped,
    bootstrap::inspect_touch_device,
    types::{TouchMode, TouchRange, TouchSlot},
};

pub(super) struct TouchState {
    mode: TouchMode,
    current_slot: usize,
    slots: Vec<TouchSlot>,
    slot_limit: Option<usize>,
    x_range: TouchRange,
    y_range: TouchRange,
}

impl TouchState {
    pub(super) fn from_device(device: &Device) -> Bootstrapped<Self> {
        let Some(bootstrap) = inspect_touch_device(device) else {
            return Bootstrapped::new(Self::disabled());
        };

        Bootstrapped::with_warnings(
            Self::from_parts(
                bootstrap.mode,
                bootstrap.slot_limit,
                bootstrap.x_range,
                bootstrap.y_range,
            ),
            bootstrap.startup_warnings,
        )
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
        let slot_limit = mode.slot_limit(slot_limit);
        let mut slots = vec![TouchSlot::default(); mode.slot_count(slot_limit)];
        initialize_slots(&mut slots, mode);

        Self {
            mode,
            current_slot: 0,
            slots,
            slot_limit,
            x_range,
            y_range,
        }
    }

    pub(super) fn enabled(&self) -> bool {
        self.x_range.is_known() && self.y_range.is_known()
    }

    pub(super) fn is_touch_device(&self) -> bool {
        self.mode.is_touch_device()
    }

    pub(super) fn x_range(&self) -> Option<(i32, i32)> {
        self.x_range.range()
    }

    pub(super) fn y_range(&self) -> Option<(i32, i32)> {
        self.y_range.range()
    }

    pub(super) fn ranges(&self) -> Option<((i32, i32), (i32, i32))> {
        Some((self.x_range()?, self.y_range()?))
    }

    #[cfg(test)]
    pub(super) fn disabled_for_tests() -> Self {
        Self::disabled()
    }

    #[cfg(test)]
    pub(super) fn touch_device_for_tests(enabled: bool) -> Self {
        let range = if enabled {
            TouchRange::fixed(0, 100)
        } else {
            TouchRange::Unknown
        };

        Self::from_parts(
            TouchMode::SingleTouch { contact_key: None },
            None,
            range,
            range,
        )
    }
}

fn initialize_slots(slots: &mut [TouchSlot], mode: TouchMode) {
    if mode.seeds_primary_tracking() && !slots.is_empty() {
        slots[0].tracking_id = Some(0);
    }
}

#[cfg(test)]
mod tests {
    use evdev::KeyCode;

    use super::{
        TouchState, initialize_slots,
        types::{MultiTouchSlots, TouchMode, TouchRange, TouchSlot},
    };

    #[test]
    fn disabled_state_is_not_enabled_or_touch_capable() {
        let state = TouchState::disabled();

        assert!(!state.enabled());
        assert!(!state.is_touch_device());
        assert_eq!(state.ranges(), None);
        assert!(state.slots.is_empty());
    }

    #[test]
    fn ranges_require_both_axes_to_be_known() {
        let state = TouchState::from_parts(
            TouchMode::SingleTouch { contact_key: None },
            None,
            TouchRange::fixed(0, 100),
            TouchRange::Unknown,
        );

        assert_eq!(state.x_range(), Some((0, 100)));
        assert_eq!(state.y_range(), None);
        assert_eq!(state.ranges(), None);
        assert!(!state.enabled());
        assert!(state.is_touch_device());
    }

    #[test]
    fn single_touch_without_contact_key_seeds_primary_tracking() {
        let state = TouchState::from_parts(
            TouchMode::SingleTouch { contact_key: None },
            None,
            TouchRange::fixed(0, 100),
            TouchRange::fixed(0, 100),
        );

        assert_eq!(state.slots.len(), 1);
        assert_eq!(state.slots[0].tracking_id, Some(0));
        assert_eq!(state.slot_limit, Some(1));
    }

    #[test]
    fn explicit_multi_touch_uses_the_detected_slot_limit() {
        let state = TouchState::from_parts(
            TouchMode::MultiTouch {
                slots: MultiTouchSlots::Explicit,
            },
            Some(3),
            TouchRange::fixed(0, 100),
            TouchRange::fixed(0, 100),
        );

        assert_eq!(state.slots.len(), 3);
        assert_eq!(state.slot_limit, Some(3));
        assert!(state.slots.iter().all(|slot| slot.tracking_id.is_none()));
    }

    #[test]
    fn implicit_multi_touch_initializes_a_single_slot() {
        let state = TouchState::from_parts(
            TouchMode::MultiTouch {
                slots: MultiTouchSlots::ImplicitSingle,
            },
            Some(4),
            TouchRange::fixed(0, 100),
            TouchRange::fixed(0, 100),
        );

        assert_eq!(state.slots.len(), 1);
        assert_eq!(state.slot_limit, Some(1));
    }

    #[test]
    fn initialize_slots_only_seeds_tracking_for_contactless_single_touch() {
        let mut slots = vec![TouchSlot::default()];
        initialize_slots(&mut slots, TouchMode::SingleTouch { contact_key: None });
        assert_eq!(slots[0].tracking_id, Some(0));

        let mut slots = vec![TouchSlot::default()];
        initialize_slots(
            &mut slots,
            TouchMode::SingleTouch {
                contact_key: Some(KeyCode::BTN_TOUCH),
            },
        );
        assert_eq!(slots[0].tracking_id, None);
    }
}
