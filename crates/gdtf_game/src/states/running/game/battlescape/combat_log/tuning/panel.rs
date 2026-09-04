use bevy::prelude::*;
use serde::Deserialize;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct HeightLerpRate(f32);

impl HeightLerpRate {
    pub(crate) const DEFAULT: f32 = 10.0;
}

impl Default for HeightLerpRate {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct BottomClearanceLines(f32);

impl BottomClearanceLines {
    pub(crate) const DEFAULT: f32 = 0.5;
}

impl Default for BottomClearanceLines {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct PanelWidthVw(f32);

impl PanelWidthVw {
    pub(crate) const DEFAULT: f32 = 28.0;
}

impl Default for PanelWidthVw {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}
