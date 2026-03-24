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
