use bevy::prelude::*;
use serde::Deserialize;

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ScreenColor(Color);

impl ScreenColor {
        #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PanelColor(Color);

impl PanelColor {
        #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ButtonColor(Color);

impl ButtonColor {
        #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct DisabledColor(Color);

impl DisabledColor {
        #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ActiveColor(Color);

impl ActiveColor {
        #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct HoverColor(Color);

impl HoverColor {
        #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PressedColor(Color);

impl PressedColor {
        #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct TextColor(Color);

impl TextColor {
        #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderColor(Color);

impl BorderColor {
        #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderWidthVw(f32);

impl BorderWidthVw {
        #[must_use]
    pub const fn new(vw: f32) -> Self {
        Self(vw)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CornerRadiusVw(f32);

impl CornerRadiusVw {
        #[must_use]
    pub const fn new(vw: f32) -> Self {
        Self(vw)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct MarginVw(f32);

impl MarginVw {
        #[must_use]
    pub const fn new(vw: f32) -> Self {
        Self(vw)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct MarginVh(f32);

impl MarginVh {
        #[must_use]
    pub const fn new(vh: f32) -> Self {
        Self(vh)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ContentMargin {
        pub l: MarginVw,
        pub r: MarginVw,
        pub t: MarginVh,
        pub b: MarginVh,
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct FontSizePt(f32);

impl FontSizePt {
        #[must_use]
    pub const fn new(pt: f32) -> Self {
        Self(pt)
    }
}

#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct FontKey(String);

impl FontKey {
        #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(transparent)]
pub struct Srgba4([f32; 4]);

impl Srgba4 {
        #[must_use]
    pub const fn new(channels: [f32; 4]) -> Self {
        Self(channels)
    }

        pub(super) const fn into_color(self) -> Color {
        let [r, g, b, a] = self.0;
        Color::srgba(r, g, b, a)
    }
}

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct MarginSpec {
        pub left:   f32,
        pub right:  f32,
        pub top:    f32,
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
