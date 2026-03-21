use std::collections::HashMap;

use evdev::{
    AbsoluteAxisCode, AttributeSetRef, Device, EventType, InputEvent, KeyCode, RelativeAxisCode,
};

use crate::device::monitor::{ComponentBootstrap, config, math};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum InputTypeId {
    Abs,
    Rel,
    Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct InputId {
    pub(crate) kind: InputTypeId,
    pub(crate) code: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AbsoluteState {
    Kernel { min: i32, max: i32, value: i32 },
    Fallback { min: i32, max: i32, value: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InputKind {
    Absolute(AbsoluteState),
    Relative(i32),
    Button(bool),
}

impl InputKind {
    pub(crate) fn normalized(&self) -> f64 {
        match *self {
            Self::Absolute(AbsoluteState::Kernel { min, max, value })
            | Self::Absolute(AbsoluteState::Fallback { min, max, value }) => {
                math::normalize_range(value, min, max)
            }
            Self::Relative(value) => math::normalize_wrapped(value, config::RELATIVE_DISPLAY_RANGE),
            Self::Button(pressed) => (pressed as u8) as f64,
        }
    }

    pub(crate) fn display_label(&self) -> String {
        match self {
            Self::Absolute(AbsoluteState::Kernel { value, .. })
            | Self::Absolute(AbsoluteState::Fallback { value, .. }) => value.to_string(),
            Self::Relative(value) => {
                math::wrapped_value(*value, config::RELATIVE_DISPLAY_RANGE).to_string()
            }
            Self::Button(pressed) => button_label(*pressed).to_string(),
        }
    }

    pub(crate) fn update(&mut self, event: &InputEvent) {
        let value = event.value();
        match (self, event.event_type()) {
            (
                Self::Absolute(AbsoluteState::Kernel { value: current, .. })
                | Self::Absolute(AbsoluteState::Fallback { value: current, .. }),
                EventType::ABSOLUTE,
            ) => *current = value,
            (Self::Relative(v), EventType::RELATIVE) => {
                *v = v.saturating_add(value);
            }
            (Self::Button(pressed), EventType::KEY) => *pressed = value != 0,
            _ => {}
        }
    }
}

fn button_label(pressed: bool) -> &'static str {
    if pressed { "ON" } else { "OFF" }
}

#[derive(Debug, Clone)]
pub(crate) struct DeviceInput {
    pub(crate) name: String,
    pub(crate) input_type: InputKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AbsoluteAxis {
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) value: i32,
}

pub(crate) type InputSlice<'a> = &'a [DeviceInput];

pub(crate) struct InputCollection {
    absolute: Vec<DeviceInput>,
    relative: Vec<DeviceInput>,
    buttons: Vec<DeviceInput>,
    by_event: HashMap<InputId, InputLocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AxisSnapshot {
    min: i32,
    max: i32,
    value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputLocation {
    Absolute(usize),
    Relative(usize),
    Button(usize),
}

impl InputCollection {
    pub(crate) fn from_device(device: &Device) -> ComponentBootstrap<Self> {
        let mut absolute = Vec::new();
        let mut relative = Vec::new();
        let mut buttons = Vec::new();
        let mut startup_warnings = Vec::new();

        let abs_state = if device.supported_absolute_axes().is_some() {
            match device.get_abs_state() {
                Ok(state) => Some(state),
                Err(err) => {
                    startup_warnings.push(format!(
                        "unable to load absolute axis state; using fallback defaults until events arrive: {err}"
                    ));
                    None
                }
            }
        } else {
            None
        };

        let key_state = if device.supported_keys().is_some() {
            match device.get_key_state() {
                Ok(state) => Some(state),
                Err(err) => {
                    startup_warnings.push(format!(
                        "unable to load key/button state; buttons start released until events arrive: {err}"
                    ));
                    None
                }
            }
        } else {
            None
        };

        // Collect absolute axes
        if let Some(axes) = device.supported_absolute_axes() {
            for axis in axes.iter() {
                let code = axis.0;
                absolute.push((
                    code,
                    DeviceInput {
                        name: format!("{:?}", AbsoluteAxisCode(code)).to_lowercase(),
                        input_type: InputKind::Absolute(absolute_state_from_snapshot(
                            abs_state.as_ref().and_then(|state| {
                                state.get(code as usize).map(|info| AxisSnapshot {
                                    min: info.minimum,
                                    max: info.maximum,
                                    value: info.value,
                                })
                            }),
                        )),
                    },
                ));
            }
        }

        // Collect relative axes
        if let Some(axes) = device.supported_relative_axes() {
            for axis in axes.iter() {
                let code = axis.0;
                relative.push((
                    code,
                    DeviceInput {
                        name: format!("{:?}", RelativeAxisCode(code)).to_lowercase(),
                        input_type: InputKind::Relative(0),
                    },
                ));
            }
        }

        // Collect buttons
        if let Some(keys) = device.supported_keys() {
            for key in keys.iter() {
                if is_touch_contact_button(key) {
                    continue;
                }
                let code = key.0;
                buttons.push((
                    code,
                    DeviceInput {
                        name: strip_btn_prefix(&format!("{key:?}").to_lowercase()),
                        input_type: InputKind::Button(is_key_pressed(key, key_state.as_deref())),
                    },
                ));
            }
        }

        ComponentBootstrap {
            value: Self::from_entries(absolute, relative, buttons),
            startup_warnings,
        }
    }

    pub(crate) fn handle_event(&mut self, event: &InputEvent) {
        if let Some(id) = InputId::from_event(event)
            && let Some(location) = self.by_event.get(&id).copied()
            && let Some(input) = self.input_mut(location)
        {
            input.input_type.update(event);
        }
    }

    pub(crate) fn reset_relative_axes(&mut self) {
        for input in &mut self.relative {
            if let InputKind::Relative(v) = &mut input.input_type {
                *v = 0;
            }
        }
    }

    pub(crate) fn absolute_axis(&self, code: AbsoluteAxisCode) -> Option<AbsoluteAxis> {
        let location = self.by_event.get(&InputId::absolute(code.0)).copied()?;
        let input = self.input(location)?;
        match input.input_type {
            InputKind::Absolute(AbsoluteState::Kernel { min, max, value })
            | InputKind::Absolute(AbsoluteState::Fallback { min, max, value }) => {
                Some(AbsoluteAxis { min, max, value })
            }
            _ => None,
        }
    }

    pub(crate) fn absolute_axis_pair(
        &self,
        x: AbsoluteAxisCode,
        y: AbsoluteAxisCode,
    ) -> Option<(AbsoluteAxis, AbsoluteAxis)> {
        Some((self.absolute_axis(x)?, self.absolute_axis(y)?))
    }

    pub(crate) fn absolute_inputs(&self) -> &[DeviceInput] {
        &self.absolute
    }

    pub(crate) fn relative_inputs(&self) -> &[DeviceInput] {
        &self.relative
    }

    pub(crate) fn button_inputs(&self) -> &[DeviceInput] {
        &self.buttons
    }

    fn from_entries(
        absolute: Vec<(u16, DeviceInput)>,
        relative: Vec<(u16, DeviceInput)>,
        buttons: Vec<(u16, DeviceInput)>,
    ) -> Self {
        let mut by_event = HashMap::new();

        let absolute = Self::sorted_inputs(
            absolute,
            InputId::absolute,
            InputLocation::Absolute,
            &mut by_event,
        );
        let relative = Self::sorted_inputs(
            relative,
            InputId::relative,
            InputLocation::Relative,
            &mut by_event,
        );
        let buttons =
            Self::sorted_inputs(buttons, InputId::key, InputLocation::Button, &mut by_event);

        Self {
            absolute,
            relative,
            buttons,
            by_event,
        }
    }

    fn sorted_inputs(
        mut entries: Vec<(u16, DeviceInput)>,
        make_id: impl Fn(u16) -> InputId,
        locate: impl Fn(usize) -> InputLocation,
        by_event: &mut HashMap<InputId, InputLocation>,
    ) -> Vec<DeviceInput> {
        entries.sort_unstable_by_key(|(code, _)| *code);
        entries
            .into_iter()
            .enumerate()
            .map(|(index, (code, input))| {
                by_event.insert(make_id(code), locate(index));
                input
            })
            .collect()
    }

    fn input(&self, location: InputLocation) -> Option<&DeviceInput> {
        match location {
            InputLocation::Absolute(index) => self.absolute.get(index),
            InputLocation::Relative(index) => self.relative.get(index),
            InputLocation::Button(index) => self.buttons.get(index),
        }
    }

    fn input_mut(&mut self, location: InputLocation) -> Option<&mut DeviceInput> {
        match location {
            InputLocation::Absolute(index) => self.absolute.get_mut(index),
            InputLocation::Relative(index) => self.relative.get_mut(index),
            InputLocation::Button(index) => self.buttons.get_mut(index),
        }
    }
}

impl InputId {
    pub(crate) fn new(kind: InputTypeId, code: u16) -> Self {
        Self { kind, code }
    }

    pub(crate) fn absolute(code: u16) -> Self {
        Self::new(InputTypeId::Abs, code)
    }

    pub(crate) fn relative(code: u16) -> Self {
        Self::new(InputTypeId::Rel, code)
    }

    pub(crate) fn key(code: u16) -> Self {
        Self::new(InputTypeId::Key, code)
    }

    pub(crate) fn from_event(event: &InputEvent) -> Option<Self> {
        let kind = match event.event_type() {
            EventType::ABSOLUTE => InputTypeId::Abs,
            EventType::RELATIVE => InputTypeId::Rel,
            EventType::KEY => InputTypeId::Key,
            _ => return None,
        };

        Some(Self::new(kind, event.code()))
    }
}

fn absolute_state_from_snapshot(snapshot: Option<AxisSnapshot>) -> AbsoluteState {
    if let Some(snapshot) = snapshot {
        AbsoluteState::Kernel {
            min: snapshot.min,
            max: snapshot.max,
            value: snapshot.value,
        }
    } else {
        AbsoluteState::Fallback {
            min: config::DEFAULT_AXIS_RANGE.0,
            max: config::DEFAULT_AXIS_RANGE.1,
            value: 0,
        }
    }
}

fn is_key_pressed(code: KeyCode, key_state: Option<&AttributeSetRef<KeyCode>>) -> bool {
    key_state.is_some_and(|state| state.contains(code))
}

fn is_touch_contact_button(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::BTN_TOUCH
            | KeyCode::BTN_TOOL_FINGER
            | KeyCode::BTN_TOOL_DOUBLETAP
            | KeyCode::BTN_TOOL_TRIPLETAP
            | KeyCode::BTN_TOOL_QUADTAP
            | KeyCode::BTN_TOOL_QUINTTAP
    )
}

fn strip_btn_prefix(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("btn_") {
        rest.to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use evdev::{AttributeSet, EventType, InputEvent, KeyCode};

    use super::{
        AbsoluteState, AxisSnapshot, DeviceInput, InputCollection, InputId, InputKind,
        InputLocation, InputTypeId, absolute_state_from_snapshot, is_key_pressed,
        is_touch_contact_button,
    };
    use crate::device::monitor::config;

    #[test]
    fn absolute_state_from_snapshot_uses_kernel_values() {
        assert_eq!(
            absolute_state_from_snapshot(Some(AxisSnapshot {
                min: -10,
                max: 20,
                value: 7,
            })),
            AbsoluteState::Kernel {
                min: -10,
                max: 20,
                value: 7,
            }
        );
    }

    #[test]
    fn absolute_state_from_snapshot_uses_explicit_fallback_defaults() {
        assert_eq!(
            absolute_state_from_snapshot(None),
            AbsoluteState::Fallback {
                min: config::DEFAULT_AXIS_RANGE.0,
                max: config::DEFAULT_AXIS_RANGE.1,
                value: 0,
            }
        );
    }

    #[test]
    fn is_key_pressed_reads_initial_button_state() {
        let mut keys = AttributeSet::new();
        keys.insert(KeyCode::BTN_SOUTH);

        assert!(is_key_pressed(KeyCode::BTN_SOUTH, Some(&keys)));
        assert!(!is_key_pressed(KeyCode::BTN_EAST, Some(&keys)));
        assert!(!is_key_pressed(KeyCode::BTN_SOUTH, None));
    }

    #[test]
    fn is_touch_contact_button_filters_touch_contact_keys() {
        assert!(is_touch_contact_button(KeyCode::BTN_TOUCH));
        assert!(is_touch_contact_button(KeyCode::BTN_TOOL_FINGER));
        assert!(is_touch_contact_button(KeyCode::BTN_TOOL_DOUBLETAP));
        assert!(!is_touch_contact_button(KeyCode::BTN_LEFT));
        assert!(!is_touch_contact_button(KeyCode::BTN_SOUTH));
    }

    #[test]
    fn input_id_helpers_use_named_fields() {
        assert_eq!(
            InputId::absolute(1),
            InputId {
                kind: InputTypeId::Abs,
                code: 1,
            }
        );
        assert_eq!(
            InputId::relative(2),
            InputId {
                kind: InputTypeId::Rel,
                code: 2,
            }
        );
        assert_eq!(
            InputId::key(3),
            InputId {
                kind: InputTypeId::Key,
                code: 3,
            }
        );
    }

    fn absolute_input(name: &str, value: i32) -> DeviceInput {
        DeviceInput {
            name: name.to_string(),
            input_type: InputKind::Absolute(AbsoluteState::Kernel {
                min: -10,
                max: 10,
                value,
            }),
        }
    }

    fn relative_input(name: &str, value: i32) -> DeviceInput {
        DeviceInput {
            name: name.to_string(),
            input_type: InputKind::Relative(value),
        }
    }

    fn button_input(name: &str, pressed: bool) -> DeviceInput {
        DeviceInput {
            name: name.to_string(),
            input_type: InputKind::Button(pressed),
        }
    }

    #[test]
    fn from_entries_preserves_sorted_category_order() {
        let inputs = InputCollection::from_entries(
            vec![
                (4, absolute_input("abs_z", 0)),
                (1, absolute_input("abs_x", 0)),
            ],
            vec![
                (3, relative_input("rel_y", 0)),
                (2, relative_input("rel_x", 0)),
            ],
            vec![
                (9, button_input("east", false)),
                (1, button_input("south", false)),
            ],
        );

        assert_eq!(
            inputs
                .absolute_inputs()
                .iter()
                .map(|input| input.name.as_str())
                .collect::<Vec<_>>(),
            vec!["abs_x", "abs_z"]
        );
        assert_eq!(
            inputs
                .relative_inputs()
                .iter()
                .map(|input| input.name.as_str())
                .collect::<Vec<_>>(),
            vec!["rel_x", "rel_y"]
        );
        assert_eq!(
            inputs
                .button_inputs()
                .iter()
                .map(|input| input.name.as_str())
                .collect::<Vec<_>>(),
            vec!["south", "east"]
        );
    }

    #[test]
    fn handle_event_routes_updates_to_each_category() {
        let mut inputs = InputCollection::from_entries(
            vec![(0, absolute_input("abs_x", 1))],
            vec![(1, relative_input("rel_x", 2))],
            vec![(2, button_input("south", false))],
        );

        inputs.handle_event(&InputEvent::new(EventType::ABSOLUTE.0, 0, 7));
        inputs.handle_event(&InputEvent::new(EventType::RELATIVE.0, 1, 3));
        inputs.handle_event(&InputEvent::new(EventType::KEY.0, 2, 1));

        assert_eq!(
            inputs.absolute_inputs()[0].input_type,
            InputKind::Absolute(AbsoluteState::Kernel {
                min: -10,
                max: 10,
                value: 7,
            })
        );
        assert_eq!(
            inputs.relative_inputs()[0].input_type,
            InputKind::Relative(5)
        );
        assert_eq!(
            inputs.button_inputs()[0].input_type,
            InputKind::Button(true)
        );
    }

    #[test]
    fn reset_relative_axes_only_clears_relative_values() {
        let mut inputs = InputCollection::from_entries(
            vec![(0, absolute_input("abs_x", 4))],
            vec![
                (1, relative_input("rel_x", 5)),
                (2, relative_input("rel_y", -2)),
            ],
            vec![(3, button_input("south", true))],
        );

        inputs.reset_relative_axes();

        assert_eq!(
            inputs
                .relative_inputs()
                .iter()
                .map(|input| &input.input_type)
                .collect::<Vec<_>>(),
            vec![&InputKind::Relative(0), &InputKind::Relative(0)]
        );
        assert_eq!(
            inputs.absolute_inputs()[0].input_type,
            InputKind::Absolute(AbsoluteState::Kernel {
                min: -10,
                max: 10,
                value: 4,
            })
        );
        assert_eq!(
            inputs.button_inputs()[0].input_type,
            InputKind::Button(true)
        );
    }

    #[test]
    fn absolute_axis_pair_reads_absolute_inputs_from_event_index() {
        let inputs = InputCollection {
            absolute: vec![absolute_input("abs_x", 4), absolute_input("abs_y", -3)],
            relative: Vec::new(),
            buttons: Vec::new(),
            by_event: HashMap::from([
                (InputId::absolute(0), InputLocation::Absolute(0)),
                (InputId::absolute(1), InputLocation::Absolute(1)),
            ]),
        };

        let pair = inputs.absolute_axis_pair(
            evdev::AbsoluteAxisCode::ABS_X,
            evdev::AbsoluteAxisCode::ABS_Y,
        );

        assert_eq!(
            pair,
            Some((
                super::AbsoluteAxis {
                    min: -10,
                    max: 10,
                    value: 4,
                },
                super::AbsoluteAxis {
                    min: -10,
                    max: 10,
                    value: -3,
                }
            ))
        );
    }
}
