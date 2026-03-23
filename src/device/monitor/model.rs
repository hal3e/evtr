mod bootstrap;
mod types;

use std::collections::HashMap;

use evdev::{AbsoluteAxisCode, Device, InputEvent};

use crate::device::monitor::ComponentBootstrap;

use self::bootstrap::collect_device_inputs;
pub(crate) use self::types::{AbsoluteAxis, DeviceInput, InputId, InputKind, InputSlice};

pub(crate) struct InputCollection {
    absolute: Vec<DeviceInput>,
    relative: Vec<DeviceInput>,
    buttons: Vec<DeviceInput>,
    by_event: HashMap<InputId, InputLocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputLocation {
    Absolute(usize),
    Relative(usize),
    Button(usize),
}

impl InputCollection {
    pub(crate) fn from_device(device: &Device) -> ComponentBootstrap<Self> {
        let entries = collect_device_inputs(device);

        ComponentBootstrap {
            value: Self::from_entries(entries.absolute, entries.relative, entries.buttons),
            startup_warnings: entries.startup_warnings,
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
            if let InputKind::Relative(value) = &mut input.input_type {
                *value = 0;
            }
        }
    }

    pub(crate) fn absolute_axis(&self, code: AbsoluteAxisCode) -> Option<AbsoluteAxis> {
        let location = self.by_event.get(&InputId::absolute(code.0)).copied()?;
        let input = self.input(location)?;
        match input.input_type {
            InputKind::Absolute(state) => Some(AbsoluteAxis {
                min: state.min,
                max: state.max,
                value: state.value,
            }),
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use evdev::{EventType, InputEvent};

    use super::{DeviceInput, InputCollection, InputId, InputKind, InputLocation};
    use crate::device::monitor::model::types::AbsoluteState;

    fn absolute_input(name: &str, value: i32) -> DeviceInput {
        DeviceInput {
            name: name.to_string(),
            input_type: InputKind::Absolute(AbsoluteState::kernel(-10, 10, value)),
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
            InputKind::Absolute(AbsoluteState::kernel(-10, 10, 7))
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
            InputKind::Absolute(AbsoluteState::kernel(-10, 10, 4))
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
