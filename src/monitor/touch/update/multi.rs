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
