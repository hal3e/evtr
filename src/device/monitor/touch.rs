use evdev::{AbsoluteAxisCode, Device, EventType, InputEvent, KeyCode, PropType};

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
    SingleTouch { has_button: bool },
}

impl TouchState {
    pub(crate) fn from_device(device: &Device) -> Self {
        let Some(axes) = device.supported_absolute_axes() else {
            return Self::disabled();
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
        let has_touch_keys = device.supported_keys().is_some_and(|keys| {
            keys.contains(KeyCode::BTN_TOUCH) || keys.contains(KeyCode::BTN_TOOL_FINGER)
        });

        let mode = if supports_mt_x && supports_mt_y {
            TouchMode::MultiTouch {
                has_slot: supports_slot,
            }
        } else if supports_abs_x && supports_abs_y && (has_touch_props || has_touch_keys) {
            TouchMode::SingleTouch {
                has_button: has_touch_keys,
            }
        } else {
            TouchMode::None
        };

        if matches!(mode, TouchMode::None) {
            return Self::disabled();
        }

        let mut x_range = None;
        let mut y_range = None;
        let mut slot_max = None;
        let (x_axis, y_axis) = match mode {
            TouchMode::MultiTouch { .. } => (
                AbsoluteAxisCode::ABS_MT_POSITION_X,
                AbsoluteAxisCode::ABS_MT_POSITION_Y,
            ),
            TouchMode::SingleTouch { .. } => (AbsoluteAxisCode::ABS_X, AbsoluteAxisCode::ABS_Y),
            TouchMode::None => return Self::disabled(),
        };

        if let Ok(absinfo) = device.get_absinfo() {
            for (axis, info) in absinfo {
                match axis {
                    axis if axis == x_axis => {
                        x_range = Some((info.minimum(), info.maximum()));
                    }
                    axis if axis == y_axis => {
                        y_range = Some((info.minimum(), info.maximum()));
                    }
                    AbsoluteAxisCode::ABS_MT_SLOT => {
                        if matches!(mode, TouchMode::MultiTouch { has_slot: true }) {
                            slot_max = Some(info.maximum());
                        }
                    }
                    _ => {}
                }
            }
        }

        if (x_range.is_none()
            || y_range.is_none()
            || (slot_max.is_none() && matches!(mode, TouchMode::MultiTouch { has_slot: true })))
            && let Ok(abs_state) = device.get_abs_state()
        {
            if x_range.is_none()
                && let Some(info) = abs_state.get(x_axis.0 as usize)
            {
                x_range = Some((info.minimum, info.maximum));
            }
            if y_range.is_none()
                && let Some(info) = abs_state.get(y_axis.0 as usize)
            {
                y_range = Some((info.minimum, info.maximum));
            }
            if slot_max.is_none()
                && matches!(mode, TouchMode::MultiTouch { has_slot: true })
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
        if matches!(mode, TouchMode::SingleTouch { has_button: false }) && !slots.is_empty() {
            slots[0].tracking_id = Some(0);
        }

        Self {
            mode,
            current_slot: 0,
            slots,
            max_slots: slots_len,
            x_range,
            y_range,
        }
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
            (TouchMode::SingleTouch { has_button }, EventType::ABSOLUTE) => {
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
                if !*has_button {
                    self.slots[0].tracking_id = Some(0);
                }
            }
            (TouchMode::SingleTouch { has_button: true }, EventType::KEY) => {
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
