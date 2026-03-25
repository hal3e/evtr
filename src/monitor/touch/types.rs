use evdev::{AbsoluteAxisCode, AttributeSetRef, KeyCode};

#[derive(Clone, Debug, Default)]
pub(super) struct TouchSlot {
    pub(super) tracking_id: Option<i32>,
    pub(super) x: Option<i32>,
    pub(super) y: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TouchRange {
    Unknown,
    Fixed { min: i32, max: i32 },
    Observed { min: i32, max: i32 },
}

impl TouchRange {
    pub(super) fn fixed(min: i32, max: i32) -> Self {
        Self::Fixed { min, max }
    }

    pub(super) fn range(&self) -> Option<(i32, i32)> {
        match self {
            Self::Unknown => None,
            Self::Fixed { min, max } | Self::Observed { min, max } => Some((*min, *max)),
        }
    }

    pub(super) fn is_known(&self) -> bool {
        self.range().is_some()
    }

    pub(super) fn observe(&mut self, value: i32) {
        match self {
            Self::Unknown => {
                *self = Self::Observed {
                    min: value,
                    max: value,
                };
            }
            Self::Observed { min, max } => {
                *min = (*min).min(value);
                *max = (*max).max(value);
            }
            Self::Fixed { .. } => {}
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MultiTouchSlots {
    ImplicitSingle,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TouchMode {
    None,
    MultiTouch { slots: MultiTouchSlots },
    SingleTouch { contact_key: Option<KeyCode> },
}

impl TouchMode {
    pub(super) fn axes(self) -> Option<(AbsoluteAxisCode, AbsoluteAxisCode)> {
        match self {
            Self::MultiTouch { .. } => Some((
                AbsoluteAxisCode::ABS_MT_POSITION_X,
                AbsoluteAxisCode::ABS_MT_POSITION_Y,
            )),
            Self::SingleTouch { .. } => Some((AbsoluteAxisCode::ABS_X, AbsoluteAxisCode::ABS_Y)),
            Self::None => None,
        }
    }

    pub(super) fn is_touch_device(self) -> bool {
        !matches!(self, Self::None)
    }

    pub(super) fn uses_explicit_slots(self) -> bool {
        matches!(
            self,
            Self::MultiTouch {
                slots: MultiTouchSlots::Explicit
            }
        )
    }

    pub(super) fn slot_limit(self, detected_limit: Option<usize>) -> Option<usize> {
        match self {
            Self::None => Some(0),
            Self::SingleTouch { .. }
            | Self::MultiTouch {
                slots: MultiTouchSlots::ImplicitSingle,
            } => Some(1),
            Self::MultiTouch {
                slots: MultiTouchSlots::Explicit,
            } => detected_limit,
        }
    }

    pub(super) fn slot_count(self, slot_limit: Option<usize>) -> usize {
        match self {
            Self::None => 0,
            _ => slot_limit.unwrap_or(1).max(1),
        }
    }

    pub(super) fn seeds_primary_tracking(self) -> bool {
        matches!(self, Self::SingleTouch { contact_key: None })
    }
}

pub(super) fn update_tracking_id(
    slot: &mut TouchSlot,
    tracking_id: Option<i32>,
    clear_position: bool,
) {
    slot.tracking_id = tracking_id;
    if tracking_id.is_some() && clear_position {
        slot.x = None;
        slot.y = None;
    }
}

pub(super) fn preferred_touch_contact_key(keys: &AttributeSetRef<KeyCode>) -> Option<KeyCode> {
    if keys.contains(KeyCode::BTN_TOUCH) {
        Some(KeyCode::BTN_TOUCH)
    } else if keys.contains(KeyCode::BTN_TOOL_FINGER) {
        Some(KeyCode::BTN_TOOL_FINGER)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use evdev::{AbsoluteAxisCode, AttributeSet, KeyCode};

    use super::{MultiTouchSlots, TouchMode, TouchRange, preferred_touch_contact_key};

    #[test]
    fn touch_range_observe_tracks_min_and_max() {
        let mut range = TouchRange::Unknown;

        range.observe(10);
        range.observe(4);
        range.observe(12);

        assert_eq!(range.range(), Some((4, 12)));
    }

    #[test]
    fn touch_range_observe_preserves_fixed_bounds() {
        let mut range = TouchRange::fixed(0, 100);

        range.observe(-10);
        range.observe(120);

        assert_eq!(range.range(), Some((0, 100)));
    }

    #[test]
    fn preferred_touch_contact_key_prefers_btn_touch() {
        let mut keys = AttributeSet::new();
        keys.insert(KeyCode::BTN_TOOL_FINGER);
        keys.insert(KeyCode::BTN_TOUCH);

        assert_eq!(preferred_touch_contact_key(&keys), Some(KeyCode::BTN_TOUCH));
    }

    #[test]
    fn touch_axes_match_touch_mode() {
        assert_eq!(
            TouchMode::SingleTouch { contact_key: None }.axes(),
            Some((AbsoluteAxisCode::ABS_X, AbsoluteAxisCode::ABS_Y))
        );
        assert_eq!(
            TouchMode::MultiTouch {
                slots: MultiTouchSlots::Explicit,
            }
            .axes(),
            Some((
                AbsoluteAxisCode::ABS_MT_POSITION_X,
                AbsoluteAxisCode::ABS_MT_POSITION_Y,
            ))
        );
        assert_eq!(TouchMode::None.axes(), None);
    }

    #[test]
    fn slot_limit_uses_explicit_slot_mode() {
        assert_eq!(
            TouchMode::MultiTouch {
                slots: MultiTouchSlots::ImplicitSingle,
            }
            .slot_limit(None),
            Some(1)
        );
        assert_eq!(
            TouchMode::MultiTouch {
                slots: MultiTouchSlots::Explicit,
            }
            .slot_limit(Some(4)),
            Some(4)
        );
    }
}
