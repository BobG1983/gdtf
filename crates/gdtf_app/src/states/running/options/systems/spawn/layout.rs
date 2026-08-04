use bevy::prelude::*;

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub(super) struct OptionsGapVh(f32);

impl OptionsGapVh {
    pub(super) const SCREEN: Self = Self(1.38889);
}

pub(super) const TOGGLE_LONG_VW: f32 = 3.75;

pub(super) const TOGGLE_SHORT_VW: f32 = 1.562_5;

pub(super) const TOGGLE_PAD_VW: f32 = 0.234_375;

pub(super) const KNOB_DIAMETER_VW: f32 = 1.093_75;

pub(super) const SETTING_VALUE_MIN_WIDTH_VW: f32 = 2.5;
