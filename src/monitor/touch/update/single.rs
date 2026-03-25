use evdev::{AbsoluteAxisCode, EventType, InputEvent, KeyCode};

use super::{PositionAxis, SlotTarget, TouchState};

pub(super) fn apply(state: &mut TouchState, contact_key: Option<KeyCode>, event: &InputEvent) {
    match event.event_type() {
        EventType::ABSOLUTE => handle_absolute(state, contact_key, event),
        EventType::KEY if contact_key.is_some() => handle_key(state, event),
        _ => {}
    }
}

fn handle_absolute(state: &mut TouchState, contact_key: Option<KeyCode>, event: &InputEvent) {
    let axis = AbsoluteAxisCode(event.code());
    let value = event.value();

    match axis {
        AbsoluteAxisCode::ABS_X => {
            state.update_slot_position(SlotTarget::Primary, PositionAxis::X, value);
        }
        AbsoluteAxisCode::ABS_Y => {
            state.update_slot_position(SlotTarget::Primary, PositionAxis::Y, value);
        }
        _ => {}
    }

    if contact_key.is_none() {
        state.arm_primary_slot_without_reset();
    }
}

fn handle_key(state: &mut TouchState, event: &InputEvent) {
    let key = KeyCode(event.code());
    if matches!(key, KeyCode::BTN_TOUCH | KeyCode::BTN_TOOL_FINGER) {
        state.set_slot_tracking_id(SlotTarget::Primary, (event.value() != 0).then_some(0), true);
    }
}

#[cfg(test)]
mod tests {
    use evdev::{AbsoluteAxisCode, EventType, InputEvent, KeyCode};

    use super::apply;
    use crate::monitor::touch::{
        TouchState,
        types::{TouchMode, TouchRange},
    };

    fn abs(axis: AbsoluteAxisCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::ABSOLUTE.0, axis.0, value)
    }

    fn key(code: KeyCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::KEY.0, code.0, value)
    }

    fn state(contact_key: Option<KeyCode>) -> TouchState {
        TouchState::from_parts(
            TouchMode::SingleTouch { contact_key },
            None,
            TouchRange::fixed(0, 100),
            TouchRange::fixed(0, 100),
        )
    }

    #[test]
    fn apply_ignores_unrelated_absolute_axes() {
        let mut state = state(None);

        apply(&mut state, None, &abs(AbsoluteAxisCode::ABS_Z, 33));

        assert!(state.active_points().is_empty());
        assert_eq!(state.x_range(), Some((0, 100)));
        assert_eq!(state.y_range(), Some((0, 100)));
    }

    #[test]
    fn apply_ignores_unrelated_key_codes() {
        let mut state = state(Some(KeyCode::BTN_TOUCH));

        apply(
            &mut state,
            Some(KeyCode::BTN_TOUCH),
            &key(KeyCode::BTN_LEFT, 1),
        );

        assert!(state.active_points().is_empty());
        assert!(state.inactive_points().is_empty());
    }

    #[test]
    fn apply_ignores_key_events_when_no_contact_key_is_configured() {
        let mut state = state(None);

        apply(&mut state, None, &key(KeyCode::BTN_TOUCH, 1));

        assert_eq!(state.active_points(), Vec::<(i32, i32)>::new());
    }

    #[test]
    fn apply_updates_x_and_y_independently() {
        let mut state = state(None);

        apply(&mut state, None, &abs(AbsoluteAxisCode::ABS_X, 12));
        assert_eq!(state.active_points(), Vec::<(i32, i32)>::new());

        apply(&mut state, None, &abs(AbsoluteAxisCode::ABS_Y, 34));
        assert_eq!(state.active_points(), vec![(12, 34)]);

        apply(&mut state, None, &abs(AbsoluteAxisCode::ABS_X, 56));
        assert_eq!(state.active_points(), vec![(56, 34)]);
    }

    #[test]
    fn apply_tracks_primary_contact_press_and_release() {
        let mut state = state(Some(KeyCode::BTN_TOUCH));

        apply(
            &mut state,
            Some(KeyCode::BTN_TOUCH),
            &key(KeyCode::BTN_TOUCH, 1),
        );
        apply(
            &mut state,
            Some(KeyCode::BTN_TOUCH),
            &abs(AbsoluteAxisCode::ABS_X, 10),
        );
        apply(
            &mut state,
            Some(KeyCode::BTN_TOUCH),
            &abs(AbsoluteAxisCode::ABS_Y, 20),
        );
        assert_eq!(state.active_points(), vec![(10, 20)]);

        apply(
            &mut state,
            Some(KeyCode::BTN_TOUCH),
            &key(KeyCode::BTN_TOUCH, 0),
        );
        assert!(state.active_points().is_empty());
        assert_eq!(state.inactive_points(), vec![(10, 20)]);
    }
}
