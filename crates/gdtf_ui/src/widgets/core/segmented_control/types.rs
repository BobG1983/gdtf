use bevy::prelude::*;

#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct SegmentLabel(String);

impl SegmentLabel {
        #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
}

#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct SegmentSubLabel(String);

impl SegmentSubLabel {
        #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
}

#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
pub struct SegmentColors {
        pub active_bg:   Color,
        pub active_text: Color,
        pub base_bg:     Color,
        pub base_text:   Color,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentedControl;

#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ActiveSegment(usize);

impl ActiveSegment {
                                    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Segment;

#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentIndex(usize);

impl SegmentIndex {
                    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentText;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentSubText;

#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct SegmentSelected {
        pub control: Entity,
        pub index:   SegmentIndex,
}
