use super::{
    index::{EventIndex, InputLocation},
    types::{AbsoluteAxis, DeviceInput, InputId, InputKind, InputSlice},
};

pub(super) struct InputEntries {
    absolute: Vec<(u16, DeviceInput)>,
    relative: Vec<(u16, DeviceInput)>,
    buttons: Vec<(u16, DeviceInput)>,
}

impl InputEntries {
    pub(super) fn new(
        absolute: Vec<(u16, DeviceInput)>,
        relative: Vec<(u16, DeviceInput)>,
        buttons: Vec<(u16, DeviceInput)>,
    ) -> Self {
        Self {
            absolute,
            relative,
            buttons,
        }
    }

    pub(super) fn into_buckets_and_index(self) -> (InputBuckets, EventIndex) {
        let mut event_index = EventIndex::new();

        let absolute = sorted_inputs(
            self.absolute,
            InputId::absolute,
            InputLocation::Absolute,
            &mut event_index,
        );
        let relative = sorted_inputs(
            self.relative,
            InputId::relative,
            InputLocation::Relative,
            &mut event_index,
        );
        let buttons = sorted_inputs(
            self.buttons,
            InputId::key,
            InputLocation::Button,
            &mut event_index,
        );

        (
            InputBuckets {
                absolute,
                relative,
                buttons,
            },
            event_index,
        )
    }
}

pub(super) struct InputBuckets {
    absolute: Vec<DeviceInput>,
    relative: Vec<DeviceInput>,
    buttons: Vec<DeviceInput>,
}

impl InputBuckets {
    pub(super) fn input(&self, location: InputLocation) -> Option<&DeviceInput> {
        match location {
            InputLocation::Absolute(index) => self.absolute.get(index),
            InputLocation::Relative(index) => self.relative.get(index),
            InputLocation::Button(index) => self.buttons.get(index),
        }
    }

    pub(super) fn input_mut(&mut self, location: InputLocation) -> Option<&mut DeviceInput> {
        match location {
            InputLocation::Absolute(index) => self.absolute.get_mut(index),
            InputLocation::Relative(index) => self.relative.get_mut(index),
            InputLocation::Button(index) => self.buttons.get_mut(index),
        }
    }

    pub(super) fn reset_relative_axes(&mut self) {
        for input in &mut self.relative {
            if let InputKind::Relative(value) = &mut input.input_type {
                *value = 0;
            }
        }
    }

    pub(super) fn absolute_axis(&self, location: InputLocation) -> Option<AbsoluteAxis> {
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

    pub(super) fn absolute_inputs(&self) -> InputSlice<'_> {
        &self.absolute
    }

    pub(super) fn relative_inputs(&self) -> InputSlice<'_> {
        &self.relative
    }

    pub(super) fn button_inputs(&self) -> InputSlice<'_> {
        &self.buttons
    }
}

fn sorted_inputs(
    mut entries: Vec<(u16, DeviceInput)>,
    make_id: impl Fn(u16) -> InputId,
    locate: impl Fn(usize) -> InputLocation,
    event_index: &mut EventIndex,
) -> Vec<DeviceInput> {
    entries.sort_unstable_by_key(|(code, _)| *code);
    entries
        .into_iter()
        .enumerate()
        .map(|(index, (code, input))| {
            event_index.insert(make_id(code), locate(index));
            input
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{InputBuckets, InputEntries, sorted_inputs};
    use crate::monitor::model::{
        AbsoluteState, DeviceInput, InputId, InputKind,
        index::{EventIndex, InputLocation},
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

    fn buckets() -> InputBuckets {
        InputBuckets {
            absolute: vec![absolute_input("abs_x", 4)],
            relative: vec![relative_input("rel_x", 3)],
            buttons: vec![button_input("south", true)],
        }
    }

    #[test]
    fn into_buckets_and_index_sorts_inputs_and_records_locations() {
        let entries = InputEntries::new(
            vec![
                (4, absolute_input("abs_z", 0)),
                (1, absolute_input("abs_x", 1)),
            ],
            vec![
                (3, relative_input("rel_y", 0)),
                (2, relative_input("rel_x", 2)),
            ],
            vec![
                (9, button_input("east", false)),
                (1, button_input("south", true)),
            ],
        );

        let (buckets, index) = entries.into_buckets_and_index();

        assert_eq!(buckets.absolute[0].name, "abs_x");
        assert_eq!(buckets.absolute[1].name, "abs_z");
        assert_eq!(buckets.relative[0].name, "rel_x");
        assert_eq!(buckets.buttons[0].name, "south");
        assert_eq!(
            index.location_for(InputId::absolute(1)),
            Some(InputLocation::Absolute(0))
        );
        assert_eq!(
            index.location_for(InputId::relative(3)),
            Some(InputLocation::Relative(1))
        );
        assert_eq!(
            index.location_for(InputId::key(9)),
            Some(InputLocation::Button(1))
        );
    }

    #[test]
    fn input_and_input_mut_use_the_requested_bucket_location() {
        let mut buckets = buckets();

        assert_eq!(
            buckets
                .input(InputLocation::Absolute(0))
                .map(|input| input.name.as_str()),
            Some("abs_x")
        );
        assert_eq!(
            buckets
                .input(InputLocation::Relative(0))
                .map(|input| input.name.as_str()),
            Some("rel_x")
        );
        assert_eq!(
            buckets
                .input(InputLocation::Button(0))
                .map(|input| input.name.as_str()),
            Some("south")
        );
        assert!(buckets.input(InputLocation::Button(1)).is_none());

        if let Some(input) = buckets.input_mut(InputLocation::Relative(0)) {
            input.name = "rel_dx".to_string();
        }

        assert_eq!(
            buckets
                .input(InputLocation::Relative(0))
                .map(|input| input.name.as_str()),
            Some("rel_dx")
        );
    }

    #[test]
    fn absolute_axis_returns_none_for_non_absolute_locations() {
        let buckets = buckets();

        assert_eq!(
            buckets.absolute_axis(InputLocation::Absolute(0)),
            Some(crate::monitor::model::AbsoluteAxis {
                min: -10,
                max: 10,
                value: 4,
            })
        );
        assert_eq!(buckets.absolute_axis(InputLocation::Relative(0)), None);
        assert_eq!(buckets.absolute_axis(InputLocation::Button(0)), None);
    }

    #[test]
    fn reset_relative_axes_only_clears_relative_bucket_values() {
        let mut buckets = buckets();

        buckets.reset_relative_axes();

        assert_eq!(buckets.relative[0].input_type, InputKind::Relative(0));
        assert_eq!(
            buckets.absolute[0].input_type,
            InputKind::Absolute(AbsoluteState::kernel(-10, 10, 4))
        );
        assert_eq!(buckets.buttons[0].input_type, InputKind::Button(true));
    }

    #[test]
    fn sorted_inputs_returns_grouped_order_and_populates_the_event_index() {
        let mut index = EventIndex::new();

        let inputs = sorted_inputs(
            vec![
                (5, relative_input("rel_z", 0)),
                (1, relative_input("rel_x", 0)),
                (3, relative_input("rel_y", 0)),
            ],
            InputId::relative,
            InputLocation::Relative,
            &mut index,
        );

        assert_eq!(
            inputs
                .iter()
                .map(|input| input.name.as_str())
                .collect::<Vec<_>>(),
            vec!["rel_x", "rel_y", "rel_z"]
        );
        assert_eq!(
            index.location_for(InputId::relative(1)),
            Some(InputLocation::Relative(0))
        );
        assert_eq!(
            index.location_for(InputId::relative(3)),
            Some(InputLocation::Relative(1))
        );
        assert_eq!(
            index.location_for(InputId::relative(5)),
            Some(InputLocation::Relative(2))
        );
    }
}
