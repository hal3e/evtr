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
}

fn initialize_slots(slots: &mut [TouchSlot], mode: TouchMode) {
    if mode.seeds_primary_tracking() && !slots.is_empty() {
        slots[0].tracking_id = Some(0);
    }
}
