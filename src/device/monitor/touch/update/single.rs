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
