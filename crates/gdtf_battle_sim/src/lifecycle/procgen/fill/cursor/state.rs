use bevy::prelude::Deref;

pub(in crate::lifecycle::procgen) enum FillStep {
        Placed,
        Exhausted,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ScatterSlots(usize);

impl ScatterSlots {
        pub(super) const fn new(count: usize) -> Self {
        Self(count)
    }
}

pub(super) struct ScatterState {
        pub(super) slots_remaining: ScatterSlots,
}

pub(super) enum FillPass {
        Large,
        Small,
        Scatter(ScatterState),
        Exhausted,
}

pub(super) enum BucketKind {
        Large,
        Small,
}

pub(super) enum SubPassStep {
        Placed,
        Done,
}
