//! The typed leaf values a theme is built from: color / relative-length /
//! font-size / font-key newtypes, the per-edge content margin, and the
//! wire-shaped `Srgba4` / `MarginSpec` that resolve into them.
//!
//! Each newtype keeps a **private** inner field (no-bare-types rule 5): the
//! sibling spec / runtime / fallback submodules construct it through the `new`
//! constructor and read it through the derived [`Deref`], so the raw value is
//! touched only inside this module.

use bevy::prelude::*;
use serde::Deserialize;

/// The full-screen backdrop fill color (the screen behind all UI).
///
/// Named [`ScreenColor`] rather than `BackgroundColor` to avoid clashing with
/// `bevy::ui::BackgroundColor`, the component `apply_theme` writes this into.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ScreenColor(Color);

impl ScreenColor {
    /// Wrap a resolved backdrop fill [`Color`].
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// The resting fill color of a themed panel box.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PanelColor(Color);

impl PanelColor {
    /// Wrap a resolved panel fill [`Color`].
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// The resting fill color of a themed button.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ButtonColor(Color);

impl ButtonColor {
    /// Wrap a resolved button fill [`Color`].
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// The flat background fill of a disabled / non-interactive button.
///
/// A deliberately muted color so a disabled control reads as inert and visibly
/// distinct from the active button fill — an explicit, data-driven value, not a
/// computed alpha-dim.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct DisabledColor(Color);

impl DisabledColor {
    /// Wrap a resolved disabled-button fill [`Color`].
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// The flat background fill of an **active / toggled-on** button (GTW-253).
///
/// The look an [`ActiveButton`](crate::ActiveButton) shows while its
/// toggle is ON — e.g. the Aim button while the selected ganger is aiming. A
/// distinct, data-driven value, NOT the [`PressedColor`] (which is momentary
/// click-feedback, a different meaning) nor the [`DisabledColor`]: an active
/// button reads as a persistently-engaged state, so it earns its own color.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ActiveColor(Color);

impl ActiveColor {
    /// Wrap a resolved active-button fill [`Color`].
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// The button fill shown while a themed button is hovered.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct HoverColor(Color);

impl HoverColor {
    /// Wrap a resolved hovered-button fill [`Color`].
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// The button fill shown while a themed button is pressed.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PressedColor(Color);

impl PressedColor {
    /// Wrap a resolved pressed-button fill [`Color`].
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Foreground color of themed text (labels, button captions, titles, rich text).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct TextColor(Color);

impl TextColor {
    /// Wrap a resolved themed-text foreground [`Color`].
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Color of a themed box's border stroke (a panel or a button border).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderColor(Color);

impl BorderColor {
    /// Wrap a resolved border-stroke [`Color`].
    #[must_use]
    pub const fn new(color: Color) -> Self {
        Self(color)
    }
}

/// Width of a themed box's border stroke, as a fraction of the window WIDTH
/// (`Vw`, calibrated to the 1280x720 reference window — GTW-296).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderWidthVw(f32);

impl BorderWidthVw {
    /// Wrap a border-width window-WIDTH fraction (`Vw`).
    #[must_use]
    pub const fn new(vw: f32) -> Self {
        Self(vw)
    }
}

/// Corner radius of a themed box, as a fraction of the window WIDTH (`Vw`, the
/// same axis as the border so a `2px`/`5px` border/radius pair keeps its ratio —
/// GTW-296).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CornerRadiusVw(f32);

impl CornerRadiusVw {
    /// Wrap a corner-radius window-WIDTH fraction (`Vw`).
    #[must_use]
    pub const fn new(vw: f32) -> Self {
        Self(vw)
    }
}

/// A horizontal (left / right) content-margin edge inset, as a fraction of the
/// window WIDTH (`Vw` — GTW-296).
///
/// One newtype shared by the LEFT + RIGHT edges of [`ContentMargin`]: both are
/// the same *kind* of value (a horizontal `Vw` inset), distinguished by their
/// field, so they share a type. The vertical edges use the separate [`MarginVh`]
/// so each axis tracks the matching window dimension on resize.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct MarginVw(f32);

impl MarginVw {
    /// Wrap a horizontal content-margin window-WIDTH fraction (`Vw`).
    #[must_use]
    pub const fn new(vw: f32) -> Self {
        Self(vw)
    }
}

/// A vertical (top / bottom) content-margin edge inset, as a fraction of the
/// window HEIGHT (`Vh` — GTW-296).
///
/// One newtype shared by the TOP + BOTTOM edges of [`ContentMargin`]: both are
/// the same *kind* of value (a vertical `Vh` inset), distinguished by their
/// field, so they share a type. The horizontal edges use the separate
/// [`MarginVw`] so each axis tracks the matching window dimension on resize.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct MarginVh(f32);

impl MarginVh {
    /// Wrap a vertical content-margin window-HEIGHT fraction (`Vh`).
    #[must_use]
    pub const fn new(vh: f32) -> Self {
        Self(vh)
    }
}

/// Inner padding between a themed box's border and its content, per edge.
///
/// The horizontal edges are `Vw` (window-width fractions), the vertical edges are
/// `Vh` (window-height fractions), so each axis tracks the matching window
/// dimension on resize (GTW-296).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ContentMargin {
    /// Left edge inset (`Vw`).
    pub l: MarginVw,
    /// Right edge inset (`Vw`).
    pub r: MarginVw,
    /// Top edge inset (`Vh`).
    pub t: MarginVh,
    /// Bottom edge inset (`Vh`).
    pub b: MarginVh,
}

/// Text size for a themed text role, in typographic points.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct FontSizePt(f32);

impl FontSizePt {
    /// Wrap a themed-text size in typographic points.
    #[must_use]
    pub const fn new(pt: f32) -> Self {
        Self(pt)
    }
}

/// Identifier of a theme font asset — a loose font's path under `assets/`.
///
/// On disk this is the path string (e.g. `"fonts/Alegreya-Variable.ttf"`); at
/// runtime each text-bearing sub-theme carries the resolved [`Handle<Font>`] for
/// its chosen font (its override, or the theme default).
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct FontKey(String);

impl FontKey {
    /// Wrap a loose font-path key (e.g. `"fonts/Alegreya-Variable.ttf"`).
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

/// A non-premultiplied sRGB color quad as it appears on disk: `(r, g, b, a)`,
/// each channel in `0.0..=1.0`.
///
/// We deserialize colors through this newtype rather than serde-on-[`Color`]
/// directly: the wire shape is a plain `[f32; 4]`, and resolution maps it into a
/// [`Color`] via [`Color::srgba`]. Keeping the on-disk type primitive-shaped
/// avoids coupling the RON schema to Bevy's internal color representation.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(transparent)]
pub struct Srgba4([f32; 4]);

impl Srgba4 {
    /// Wrap a non-premultiplied sRGB `(r, g, b, a)` quad.
    #[must_use]
    pub const fn new(channels: [f32; 4]) -> Self {
        Self(channels)
    }

    /// Resolve this on-disk color quad into a runtime sRGB [`Color`].
    pub(super) const fn into_color(self) -> Color {
        let [r, g, b, a] = self.0;
        Color::srgba(r, g, b, a)
    }
}

/// The on-disk margin shape: four named edge insets as relative-length
/// fractions (left/right `Vw`, top/bottom `Vh` — GTW-296).
///
/// Mirrors the nested `margin: (left:, right:, top:, bottom:)` RON form and
/// resolves into the runtime [`ContentMargin`]. The bare `f32` fields are the
/// wire-shape carve-out (no-bare-types rule): they are the inner-of-newtype
/// scalars deserialized from disk, mapped to typed [`MarginVw`] / [`MarginVh`]
/// edges in `resolve`.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct MarginSpec {
    /// Left edge inset, as a window-WIDTH fraction (`Vw`).
    pub left:   f32,
    /// Right edge inset, as a window-WIDTH fraction (`Vw`).
    pub right:  f32,
    /// Top edge inset, as a window-HEIGHT fraction (`Vh`).
    pub top:    f32,
    /// Bottom edge inset, as a window-HEIGHT fraction (`Vh`).
    pub bottom: f32,
}

impl MarginSpec {
    /// Resolve this on-disk margin into the runtime [`ContentMargin`]: the
    /// horizontal edges become [`MarginVw`] (window-width fractions), the
    /// vertical edges become [`MarginVh`] (window-height fractions).
    pub(super) const fn resolve(self) -> ContentMargin {
        ContentMargin {
            l: MarginVw::new(self.left),
            r: MarginVw::new(self.right),
            t: MarginVh::new(self.top),
            b: MarginVh::new(self.bottom),
        }
    }
}
