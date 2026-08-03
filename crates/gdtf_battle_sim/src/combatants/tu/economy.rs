use bevy::prelude::Deref;

use crate::ganger::{Tu, TuMax};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuAffordable(bool);

impl TuAffordable {
        #[must_use]
    pub const fn new(affordable: bool) -> Self {
        Self(affordable)
    }
}

#[must_use]
pub fn can_spend_tu(tu: &Tu, cost: Tu) -> TuAffordable {
    TuAffordable::new(**tu >= *cost)
}

pub fn spend_tu(tu: &mut Tu, cost: Tu) {
    *tu = Tu::new(tu.saturating_sub(*cost));
}

pub fn reset_tu(tu: &mut Tu, max: &TuMax) {
    *tu = Tu::new(**max);
}
