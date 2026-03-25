mod bootstrap;
mod types;
mod update;

use evdev::Device;

use crate::monitor::ComponentBootstrap;

use self::{
    bootstrap::inspect_touch_device,
    types::{TouchMode, TouchRange, TouchSlot},
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
        let slot_limit = slot_limit_for_mode(mode, slot_limit);
        let mut slots = vec![TouchSlot::default(); slots_len_for_mode(mode, slot_limit)];
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
}

fn slot_limit_for_mode(mode: TouchMode, slot_limit: Option<usize>) -> Option<usize> {
    match mode {
        TouchMode::None => Some(0),
        TouchMode::SingleTouch { .. } | TouchMode::MultiTouch { has_slot: false } => Some(1),
        TouchMode::MultiTouch { has_slot: true } => slot_limit,
    }
}

fn slots_len_for_mode(mode: TouchMode, slot_limit: Option<usize>) -> usize {
    match mode {
        TouchMode::None => 0,
        _ => slot_limit.unwrap_or(1).max(1),
    }
}

fn initialize_slots(slots: &mut [TouchSlot], mode: TouchMode) {
    if matches!(mode, TouchMode::SingleTouch { contact_key: None }) && !slots.is_empty() {
        slots[0].tracking_id = Some(0);
    }
}
