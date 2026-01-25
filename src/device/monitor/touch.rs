use evdev::{AbsoluteAxisCode, Device, EventType, InputEvent};

#[derive(Clone, Debug, Default)]
struct TouchSlot {
    tracking_id: Option<i32>,
    x: Option<i32>,
    y: Option<i32>,
}

pub(crate) struct TouchState {
    enabled: bool,
    current_slot: usize,
    slots: Vec<TouchSlot>,
    x_range: (i32, i32),
    y_range: (i32, i32),
}

impl TouchState {
    pub(crate) fn from_device(device: &Device) -> Self {
        let Some(axes) = device.supported_absolute_axes() else {
            return Self::disabled();
        };

        let supports_mt_x = axes
            .iter()
            .any(|axis| axis == AbsoluteAxisCode::ABS_MT_POSITION_X);
        let supports_mt_y = axes
            .iter()
            .any(|axis| axis == AbsoluteAxisCode::ABS_MT_POSITION_Y);
        let supports_slot = axes
            .iter()
            .any(|axis| axis == AbsoluteAxisCode::ABS_MT_SLOT);

        if !supports_mt_x || !supports_mt_y {
            return Self::disabled();
        }

        let mut x_range = None;
        let mut y_range = None;
        let mut slot_max = None;

        if let Ok(absinfo) = device.get_absinfo() {
            for (axis, info) in absinfo {
                match axis {
                    AbsoluteAxisCode::ABS_MT_POSITION_X => {
                        x_range = Some((info.minimum(), info.maximum()));
                    }
                    AbsoluteAxisCode::ABS_MT_POSITION_Y => {
                        y_range = Some((info.minimum(), info.maximum()));
                    }
                    AbsoluteAxisCode::ABS_MT_SLOT => {
                        if supports_slot {
                            slot_max = Some(info.maximum());
                        }
                    }
                    _ => {}
                }
            }
        }

        if (x_range.is_none() || y_range.is_none() || (slot_max.is_none() && supports_slot))
            && let Ok(abs_state) = device.get_abs_state()
        {
            if x_range.is_none()
                && let Some(info) = abs_state.get(AbsoluteAxisCode::ABS_MT_POSITION_X.0 as usize)
            {
                x_range = Some((info.minimum, info.maximum));
            }
            if y_range.is_none()
                && let Some(info) = abs_state.get(AbsoluteAxisCode::ABS_MT_POSITION_Y.0 as usize)
            {
                y_range = Some((info.minimum, info.maximum));
            }
            if slot_max.is_none() && supports_slot
                && let Some(info) = abs_state.get(AbsoluteAxisCode::ABS_MT_SLOT.0 as usize)
            {
                slot_max = Some(info.maximum);
            }
        }

        let Some(x_range) = x_range else {
            return Self::disabled();
        };
        let Some(y_range) = y_range else {
            return Self::disabled();
        };

        let slots_len = if supports_slot {
            slot_max.map(|max| max.max(0) as usize + 1).unwrap_or(1)
        } else {
            1
        };

        Self {
            enabled: true,
            current_slot: 0,
            slots: vec![TouchSlot::default(); slots_len],
            x_range,
            y_range,
        }
    }

    fn disabled() -> Self {
        Self {
            enabled: false,
            current_slot: 0,
            slots: Vec::new(),
            x_range: (0, 1),
            y_range: (0, 1),
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn x_range(&self) -> (i32, i32) {
        self.x_range
    }

    pub(crate) fn y_range(&self) -> (i32, i32) {
        self.y_range
    }

    pub(crate) fn active_points(&self) -> Vec<(i32, i32)> {
        if !self.enabled {
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
        if !self.enabled {
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
        if !self.enabled || event.event_type() != EventType::ABSOLUTE {
            return;
        }

        let axis = AbsoluteAxisCode(event.code());
        let value = event.value();
        match axis {
            AbsoluteAxisCode::ABS_MT_SLOT => {
                if value >= 0 {
                    self.current_slot = value as usize;
                    self.ensure_slot();
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

    fn ensure_slot(&mut self) {
        if self.current_slot >= self.slots.len() {
            self.slots
                .resize(self.current_slot + 1, TouchSlot::default());
        }
    }
}
