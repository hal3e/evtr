use evdev::{AbsoluteAxisCode, EventType, InputEvent};

use super::{PositionAxis, SlotTarget, TouchState};

pub(super) fn apply(state: &mut TouchState, event: &InputEvent) {
    if event.event_type() != EventType::ABSOLUTE {
        return;
    }

    let axis = AbsoluteAxisCode(event.code());
    let value = event.value();

    match axis {
        AbsoluteAxisCode::ABS_MT_SLOT => state.select_slot_value(value),
        AbsoluteAxisCode::ABS_MT_TRACKING_ID => {
            state.set_slot_tracking_id(SlotTarget::Current, (value >= 0).then_some(value), true);
        }
        AbsoluteAxisCode::ABS_MT_POSITION_X => {
            state.update_slot_position(SlotTarget::Current, PositionAxis::X, value);
        }
        AbsoluteAxisCode::ABS_MT_POSITION_Y => {
            state.update_slot_position(SlotTarget::Current, PositionAxis::Y, value);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use evdev::{AbsoluteAxisCode, EventType, InputEvent, KeyCode};

    use super::apply;
    use crate::monitor::touch::{
        TouchState,
        types::{MultiTouchSlots, TouchMode, TouchRange},
    };

    fn abs(axis: AbsoluteAxisCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::ABSOLUTE.0, axis.0, value)
    }

    fn key(code: KeyCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::KEY.0, code.0, value)
    }

    fn state() -> TouchState {
        TouchState::from_parts(
            TouchMode::MultiTouch {
                slots: MultiTouchSlots::Explicit,
            },
            Some(3),
            TouchRange::fixed(0, 100),
            TouchRange::fixed(0, 100),
        )
    }

    #[test]
    fn apply_ignores_non_absolute_events() {
        let mut state = state();

        apply(&mut state, &key(KeyCode::BTN_TOUCH, 1));

        assert_eq!(state.current_slot, 0);
        assert!(state.active_points().is_empty());
    }

    #[test]
    fn apply_ignores_unrelated_absolute_axes() {
        let mut state = state();

        apply(&mut state, &abs(AbsoluteAxisCode::ABS_X, 77));

        assert_eq!(state.current_slot, 0);
        assert!(state.active_points().is_empty());
    }

    #[test]
    fn apply_updates_selected_slot_tracking_and_position() {
        let mut state = state();

        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_SLOT, 2));
        assert_eq!(state.current_slot, 2);

        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_TRACKING_ID, 42));
        assert_eq!(state.slots[2].tracking_id, Some(42));
        assert_eq!(state.slots[2].x, None);
        assert_eq!(state.slots[2].y, None);

        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_POSITION_X, 11));
        assert_eq!(state.slots[2].x, Some(11));
        assert_eq!(state.slots[2].y, None);

        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_POSITION_Y, 22));
        assert_eq!(state.slots[2].y, Some(22));
        assert_eq!(state.active_points(), vec![(11, 22)]);
    }

    #[test]
    fn apply_release_keeps_last_position_as_inactive_point() {
        let mut state = state();

        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_SLOT, 1));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_TRACKING_ID, 7));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_POSITION_X, 31));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_POSITION_Y, 41));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_TRACKING_ID, -1));

        assert!(state.active_points().is_empty());
        assert_eq!(state.inactive_points(), vec![(31, 41)]);
    }

    #[test]
    fn apply_sequence_keeps_updates_scoped_to_the_selected_slot() {
        let mut state = state();

        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_SLOT, 1));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_TRACKING_ID, 100));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_POSITION_X, 40));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_POSITION_Y, 60));

        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_SLOT, 0));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_TRACKING_ID, 200));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_POSITION_X, 10));
        apply(&mut state, &abs(AbsoluteAxisCode::ABS_MT_POSITION_Y, 20));

        assert_eq!(state.active_points(), vec![(10, 20), (40, 60)]);
        assert_eq!(state.slots[0].tracking_id, Some(200));
        assert_eq!(state.slots[1].tracking_id, Some(100));
    }
}
