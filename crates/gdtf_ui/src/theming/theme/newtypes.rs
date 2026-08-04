//! Typed theme scalars (colors, sizes, margins).

use bevy::prelude::*;
use serde::Deserialize;

/// Screen background color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ScreenColor(Color);

impl ScreenColor {
    /// Wrap a color.
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Panel fill color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PanelColor(Color);

impl PanelColor {
    /// Wrap a color.
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Default button fill color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ButtonColor(Color);

impl ButtonColor {
    /// Wrap a color.
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Disabled button fill color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct DisabledColor(Color);

impl DisabledColor {
    /// Wrap a color.
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Active/selected button fill color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ActiveColor(Color);

impl ActiveColor {
    /// Wrap a color.
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Hovered button fill color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct HoverColor(Color);

impl HoverColor {
    /// Wrap a color.
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Pressed button fill color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PressedColor(Color);

impl PressedColor {
    /// Wrap a color.
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Text / caption color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct TextColor(Color);

impl TextColor {
    /// Wrap a color.
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Border stroke color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderColor(Color);

impl BorderColor {
    /// Wrap a color.
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Border width in viewport width units.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderWidthVw(f32);

impl BorderWidthVw {
    /// Wrap a vw value.
    #[must_use]
    pub const fn new(vw: f32) -> Self {
        Self(vw)
    }
}

/// Corner radius in viewport width units.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CornerRadiusVw(f32);

impl CornerRadiusVw {
    /// Wrap a vw value.
    #[must_use]
    pub const fn new(vw: f32) -> Self {
        Self(vw)
    }
}

/// Horizontal margin in viewport width units.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct MarginVw(f32);

impl MarginVw {
    /// Wrap a vw value.
    #[must_use]
    pub const fn new(vw: f32) -> Self {
        Self(vw)
    }
}

/// Vertical margin in viewport height units.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct MarginVh(f32);

impl MarginVh {
    /// Wrap a vh value.
    #[must_use]
    pub const fn new(vh: f32) -> Self {
        Self(vh)
    }
}

/// Content padding on all four sides.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ContentMargin {
    /// Left margin (vw).
    pub l: MarginVw,
    /// Right margin (vw).
    pub r: MarginVw,
    /// Top margin (vh).
    pub t: MarginVh,
    /// Bottom margin (vh).
    pub b: MarginVh,
}

/// Font size in points.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct FontSizePt(f32);

impl FontSizePt {
    /// Wrap a point size.
    #[must_use]
    pub const fn new(pt: f32) -> Self {
        Self(pt)
    }
}

/// Asset path key for a font.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct FontKey(String);

impl FontKey {
    /// Wrap a font path key.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

/// Four-channel sRGBA used in RON specs.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(transparent)]
pub struct Srgba4([f32; 4]);

impl Srgba4 {
    /// Wrap channel values.
    #[must_use]
    pub const fn new(channels: [f32; 4]) -> Self {
        Self(channels)
    }

    pub(super) const fn into_color(self) -> Color {
        let [r, g, b, a] = self.0;
        Color::srgba(r, g, b, a)
    }
}

/// Margin fields as stored in RON.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct MarginSpec {
    /// Left.
    pub left:   f32,
    /// Right.
    pub right:  f32,
    /// Top.
    pub top:    f32,
    /// Bottom.
    pub bottom: f32,
}

impl MarginSpec {
    pub(super) const fn resolve(self) -> ContentMargin {
        ContentMargin {
            l: MarginVw::new(self.left),
            r: MarginVw::new(self.right),
            t: MarginVh::new(self.top),
            b: MarginVh::new(self.bottom),
        }
    }
}
