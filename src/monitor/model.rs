mod bootstrap;
mod buckets;
mod index;
mod types;

use evdev::{AbsoluteAxisCode, Device, InputEvent};

#[cfg(test)]
pub(super) use self::types::AbsoluteState;
pub(super) use self::types::{AbsoluteAxis, DeviceInput, InputId, InputKind, InputSlice};
use self::{
    super::bootstrap::Bootstrapped,
    bootstrap::collect_device_inputs,
    buckets::{InputBuckets, InputEntries},
    index::EventIndex,
};

pub(super) struct InputCollection {
    buckets: InputBuckets,
    event_index: EventIndex,
}

impl InputCollection {
    pub(super) fn from_device(device: &Device) -> Bootstrapped<Self> {
        let entries = collect_device_inputs(device);

        Bootstrapped::with_warnings(
            Self::from_entries(entries.absolute, entries.relative, entries.buttons),
            entries.startup_warnings,
        )
    }

    pub(super) fn handle_event(&mut self, event: &InputEvent) {
        if let Some(id) = InputId::from_event(event)
            && let Some(location) = self.event_index.location_for(id)
            && let Some(input) = self.buckets.input_mut(location)
        {
            input.input_type.update(event);
        }
    }

    pub(super) fn reset_relative_axes(&mut self) {
        self.buckets.reset_relative_axes();
    }

    pub(super) fn absolute_axis(&self, code: AbsoluteAxisCode) -> Option<AbsoluteAxis> {
        let location = self.event_index.location_for(InputId::absolute(code.0))?;
        self.buckets.absolute_axis(location)
    }

    pub(super) fn absolute_axis_pair(
        &self,
        x: AbsoluteAxisCode,
        y: AbsoluteAxisCode,
    ) -> Option<(AbsoluteAxis, AbsoluteAxis)> {
        Some((self.absolute_axis(x)?, self.absolute_axis(y)?))
    }

    pub(super) fn absolute_inputs(&self) -> InputSlice<'_> {
        self.buckets.absolute_inputs()
    }

    pub(super) fn relative_inputs(&self) -> InputSlice<'_> {
        self.buckets.relative_inputs()
    }

    pub(super) fn button_inputs(&self) -> InputSlice<'_> {
        self.buckets.button_inputs()
    }

    fn from_entries(
        absolute: Vec<(u16, DeviceInput)>,
        relative: Vec<(u16, DeviceInput)>,
        buttons: Vec<(u16, DeviceInput)>,
    ) -> Self {
        let entries = InputEntries::new(absolute, relative, buttons);
        let (buckets, event_index) = entries.into_buckets_and_index();

        Self {
            buckets,
            event_index,
        }
    }

    #[cfg(test)]
    pub(super) fn from_entries_for_tests(
        absolute: Vec<(u16, DeviceInput)>,
        relative: Vec<(u16, DeviceInput)>,
        buttons: Vec<(u16, DeviceInput)>,
    ) -> Self {
        Self::from_entries(absolute, relative, buttons)
    }
}

#[cfg(test)]
mod tests {
    use evdev::{AbsoluteAxisCode, EventType, InputEvent};

    use super::{
        AbsoluteAxis, DeviceInput, InputCollection, InputId, InputKind, index::InputLocation,
        types::AbsoluteState,
    };

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
    fn from_entries_builds_event_index_for_each_bucket() {
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
            inputs.event_index.location_for(InputId::absolute(1)),
            Some(InputLocation::Absolute(0))
        );
        assert_eq!(
            inputs.event_index.location_for(InputId::absolute(4)),
            Some(InputLocation::Absolute(1))
        );
        assert_eq!(
            inputs.event_index.location_for(InputId::relative(2)),
            Some(InputLocation::Relative(0))
        );
        assert_eq!(
            inputs.event_index.location_for(InputId::key(9)),
            Some(InputLocation::Button(1))
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
        let inputs = InputCollection::from_entries(
            vec![
                (AbsoluteAxisCode::ABS_Y.0, absolute_input("abs_y", -3)),
                (AbsoluteAxisCode::ABS_X.0, absolute_input("abs_x", 4)),
            ],
            Vec::new(),
            Vec::new(),
        );

        let pair = inputs.absolute_axis_pair(AbsoluteAxisCode::ABS_X, AbsoluteAxisCode::ABS_Y);

        assert_eq!(
            pair,
            Some((
                AbsoluteAxis {
                    min: -10,
                    max: 10,
                    value: 4,
                },
                AbsoluteAxis {
                    min: -10,
                    max: 10,
                    value: -3,
                }
            ))
        );
    }
}
