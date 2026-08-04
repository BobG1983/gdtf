use bevy::prelude::*;
use serde::Deserialize;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub(crate) struct MaxVisibleLines(usize);

impl MaxVisibleLines {
    pub(crate) const DEFAULT: usize = 6;
}

impl Default for MaxVisibleLines {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct LineTtlSeconds(f32);

impl LineTtlSeconds {
    pub(crate) const DEFAULT: f32 = 5.0;

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn from_secs(secs: f32) -> Self {
        Self(secs)
    }
}

impl Default for LineTtlSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct FadeFraction(f32);

impl FadeFraction {
    pub(crate) const DEFAULT: f32 = 0.35;
}

impl Default for FadeFraction {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct LineLerpRate(f32);

impl LineLerpRate {
    pub(crate) const DEFAULT: f32 = 12.0;
}

impl Default for LineLerpRate {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct FadeInSeconds(f32);

impl FadeInSeconds {
    pub(crate) const DEFAULT: f32 = 0.18;

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn from_secs(secs: f32) -> Self {
        Self(secs)
    }
}

impl Default for FadeInSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct FadeOutSeconds(f32);

impl FadeOutSeconds {
    pub(crate) const DEFAULT: f32 = 0.9;

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn from_secs(secs: f32) -> Self {
        Self(secs)
    }
}

impl Default for FadeOutSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct LineFontPt(f32);

impl LineFontPt {
    pub(crate) const DEFAULT: f32 = 20.0;
}

impl Default for LineFontPt {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}
