//! Data-driven UI theme: the on-disk RON shape, its typed runtime form, and the
//! pure spec-to-resource resolution that bridges them.
//!
//! The theme is **data-driven** (ADR 0003): visual constants live in a loose
//! `assets/theme/*.ron` file, not in code. This module owns three layers:
//!
//! - [`GdtfThemeSpec`] — the serde-`Deserialize` mirror of the on-disk RON. It
//!   holds wire-friendly shapes ([`Srgba4`] color quads, a [`FontKey`] path
//!   string) and nothing Bevy-asset-bound, so it deserializes with no `World`.
//! - [`GdtfTheme`] — the runtime [`Resource`]. Every field is a typed newtype
//!   over a resolved value (a [`Color`], a px scalar, a loaded [`Handle<Font>`]);
//!   it carries **no** raw `Color`/`f32`/`String` domain fields.
//! - [`GdtfThemeSpec::resolve`] — the pure bridge: it consumes a spec plus an
//!   already-loaded font handle and produces a [`GdtfTheme`]. It touches no
//!   `World` and no `AssetServer`, so it is unit-testable fully headless.
//!
//! This module deliberately does **not** insert [`GdtfTheme`] at startup, own
//! the `Themed`/`apply_theme` machinery, or embed any asset. `OnEnter(Load)`
//! population is GTW-56; `Themed`/`apply_theme` is GTW-135; embedding is
//! deferred to packaging. Here we define the shapes and the resolution only.

use bevy::prelude::*;
use serde::Deserialize;

/// Background fill of a themed panel.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PanelBg(Color);

/// Foreground color of themed text (labels, button captions, rich text).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct TextColor(Color);

/// Color of a themed panel's border stroke.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderColor(Color);

/// Width of a themed panel's border stroke, in logical pixels.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderWidthPx(f32);

/// Corner radius of a themed panel, in logical pixels.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CornerRadiusPx(f32);

/// A single content-margin edge inset, in logical pixels.
///
/// One newtype shared by all four edges of [`ContentMargin`]: the four edges are
/// the same *kind* of value (a px inset), distinguished by their field, so they
/// share a type rather than each owning a near-identical one.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct MarginPx(f32);

/// Inner padding between a themed panel's border and its content, per edge.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ContentMargin {
    /// Left edge inset.
    pub l: MarginPx,
    /// Right edge inset.
    pub r: MarginPx,
    /// Top edge inset.
    pub t: MarginPx,
    /// Bottom edge inset.
    pub b: MarginPx,
}

/// Default text size for themed text, in typographic points.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct FontSizePt(f32);

/// Identifier of the theme's font asset — the loose font's path under `assets/`.
///
/// On disk this is the path string (e.g. `"fonts/Alegreya-Variable.ttf"`); at
/// runtime it is resolved by the `AssetServer` into a [`Handle<Font>`] carried
/// by [`GdtfTheme`].
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct FontKey(String);

/// Panel background shown while a themed interactive element is hovered.
///
/// **Port-introduced** (GTW-117): the Godot source theme has no hover/press
/// state colors. This is a slightly-lifted variant of [`PanelBg`], lighter than
/// [`PressBg`] and visibly distinct from the base panel fill.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct HoverBg(Color);

/// Panel background shown while a themed interactive element is pressed.
///
/// **Port-introduced** (GTW-117): the Godot source theme has no hover/press
/// state colors. This is a slightly-lifted variant of [`PanelBg`], darker than
/// [`HoverBg`] and visibly distinct from the base panel fill.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PressBg(Color);

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
    /// Resolve this on-disk color quad into a runtime sRGB [`Color`].
    const fn into_color(self) -> Color {
        let [r, g, b, a] = self.0;
        Color::srgba(r, g, b, a)
    }
}

/// The on-disk RON shape of a GDTF theme.
///
/// This is the deserialization mirror of `assets/theme/*.ron`: wire-friendly
/// fields only (color quads as [`Srgba4`], the font as a [`FontKey`] path, px
/// values as bare scalars under named fields). It carries nothing Bevy-asset-
/// bound, so `ron::from_str` into it needs no `World` and no `AssetServer`.
///
/// Resolve it into the runtime [`GdtfTheme`] with [`GdtfThemeSpec::resolve`].
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct GdtfThemeSpec {
    /// Foreground text color.
    pub text:             Srgba4,
    /// Panel background fill.
    pub panel_bg:         Srgba4,
    /// Panel border stroke color.
    pub border_color:     Srgba4,
    /// Panel border stroke width, in logical pixels.
    pub border_width_px:  f32,
    /// Panel corner radius, in logical pixels.
    pub corner_radius_px: f32,
    /// Left content-margin inset, in logical pixels.
    pub margin_left_px:   f32,
    /// Right content-margin inset, in logical pixels.
    pub margin_right_px:  f32,
    /// Top content-margin inset, in logical pixels.
    pub margin_top_px:    f32,
    /// Bottom content-margin inset, in logical pixels.
    pub margin_bottom_px: f32,
    /// Default text size, in typographic points.
    pub font_size_pt:     f32,
    /// Loose font asset path under `assets/`.
    pub font_key:         String,
    /// Hovered-state panel background (port-introduced — see [`HoverBg`]).
    pub hover_bg:         Srgba4,
    /// Pressed-state panel background (port-introduced — see [`PressBg`]).
    pub press_bg:         Srgba4,
}

impl GdtfThemeSpec {
    /// Resolve this on-disk spec into the runtime [`GdtfTheme`] resource.
    ///
    /// Pure: it consumes the spec and an already-loaded `font` handle and builds
    /// the typed runtime values. It accesses no `World` and no `AssetServer`, so
    /// it is fully unit-testable headless. The caller (GTW-56, `OnEnter(Load)`)
    /// is responsible for loading the font referenced by [`font_key`](Self) and
    /// passing the resulting handle in.
    #[must_use]
    pub fn resolve(self, font: Handle<Font>) -> GdtfTheme {
        GdtfTheme {
            text: TextColor(self.text.into_color()),
            panel_bg: PanelBg(self.panel_bg.into_color()),
            border_color: BorderColor(self.border_color.into_color()),
            border_width_px: BorderWidthPx(self.border_width_px),
            corner_radius_px: CornerRadiusPx(self.corner_radius_px),
            content_margin: ContentMargin {
                l: MarginPx(self.margin_left_px),
                r: MarginPx(self.margin_right_px),
                t: MarginPx(self.margin_top_px),
                b: MarginPx(self.margin_bottom_px),
            },
            font_size_pt: FontSizePt(self.font_size_pt),
            font_key: FontKey(self.font_key),
            font,
            hover_bg: HoverBg(self.hover_bg.into_color()),
            press_bg: PressBg(self.press_bg.into_color()),
        }
    }
}

/// The resolved, runtime GDTF UI theme.
///
/// Every field is a typed value (a newtype over a resolved [`Color`] or px
/// scalar, the [`FontKey`] path, the loaded [`Handle<Font>`]) — there are **no**
/// raw `Color`/`f32`/`String` domain fields. Built only via
/// [`GdtfThemeSpec::resolve`].
///
/// This resource is **not** inserted at startup. GTW-56 populates it during
/// `AppState::Load`; readers must guard for its absence per the project's
/// state-scoped-resource convention. `Themed`/`apply_theme` (GTW-135) consume it.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct GdtfTheme {
    /// Foreground text color.
    pub text:             TextColor,
    /// Panel background fill.
    pub panel_bg:         PanelBg,
    /// Panel border stroke color.
    pub border_color:     BorderColor,
    /// Panel border stroke width.
    pub border_width_px:  BorderWidthPx,
    /// Panel corner radius.
    pub corner_radius_px: CornerRadiusPx,
    /// Inner content padding, per edge.
    pub content_margin:   ContentMargin,
    /// Default text size.
    pub font_size_pt:     FontSizePt,
    /// Loose font path key the [`font`](Self::font) handle was loaded from.
    pub font_key:         FontKey,
    /// The loaded font asset handle.
    pub font:             Handle<Font>,
    /// Hovered-state panel background (port-introduced — see [`HoverBg`]).
    pub hover_bg:         HoverBg,
    /// Pressed-state panel background (port-introduced — see [`PressBg`]).
    pub press_bg:         PressBg,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped grimdark theme, read at compile time from the repo's loose
    /// asset so the test exercises the *shipped* file, not an inline literal.
    const SHIPPED_GRIMDARK_RON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/theme/grimdark.ron"
    ));

    /// Exact-bits color match. The shipped quad and the expected quad are the
    /// same decimal literals, so resolution is bit-exact — compare raw bit
    /// patterns rather than `==` (which clippy's `float_cmp` denies, and which
    /// would also be the wrong tool for an "exact value" assertion).
    fn assert_color_eq(actual: Color, expected: [f32; 4]) {
        let srgba = actual.to_srgba();
        let [r, g, b, a] = expected;
        assert_eq!(srgba.red.to_bits(), r.to_bits(), "red channel");
        assert_eq!(srgba.green.to_bits(), g.to_bits(), "green channel");
        assert_eq!(srgba.blue.to_bits(), b.to_bits(), "blue channel");
        assert_eq!(srgba.alpha.to_bits(), a.to_bits(), "alpha channel");
    }

    /// Exact-bits scalar match, for the same reason as [`assert_color_eq`].
    fn assert_px_eq(actual: f32, expected: f32, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    /// The shipped `grimdark.ron` deserializes into a [`GdtfThemeSpec`], and
    /// resolving it yields a [`GdtfTheme`] whose values equal the exact facts
    /// ported from the Godot source theme — plus the carried font handle and
    /// the port-introduced hover/press backgrounds.
    ///
    /// Pin-discriminating: any drift in a shipped color/scalar, a dropped field,
    /// or a broken resolution mapping fails one of these asserts. The `?` makes
    /// a deserialization failure a test failure without a denied `panic!`.
    #[test]
    fn shipped_grimdark_ron_resolves_to_exact_facts() -> Result<(), ron::error::SpannedError> {
        let spec: GdtfThemeSpec = ron::from_str(SHIPPED_GRIMDARK_RON)?;

        let theme = spec.resolve(Handle::<Font>::default());

        // Exact facts ported from ui/theme/main_theme.tres.
        assert_color_eq(*theme.text, [0.84, 0.80, 0.73, 1.0]);
        assert_color_eq(*theme.panel_bg, [0.08, 0.08, 0.10, 0.96]);
        assert_color_eq(*theme.border_color, [0.20, 0.20, 0.24, 1.0]);
        assert_px_eq(*theme.border_width_px, 1.0, "border width");
        assert_px_eq(*theme.corner_radius_px, 2.0, "corner radius");
        assert_px_eq(*theme.content_margin.l, 8.0, "margin left");
        assert_px_eq(*theme.content_margin.r, 8.0, "margin right");
        assert_px_eq(*theme.content_margin.t, 6.0, "margin top");
        assert_px_eq(*theme.content_margin.b, 6.0, "margin bottom");
        assert_px_eq(*theme.font_size_pt, 18.0, "font size");
        assert_eq!(&**theme.font_key, "fonts/Alegreya-Variable.ttf");

        // The resolved theme carries exactly the handle resolution was given.
        assert_eq!(theme.font, Handle::<Font>::default());

        Ok(())
    }

    /// The port-introduced hover/press backgrounds are present, and each is a
    /// distinct, visibly-lifted variant of the base panel fill — hover lighter
    /// than press, both different from base.
    ///
    /// Pin-discriminating: collapsing hover or press back onto the panel fill,
    /// or swapping their relative lightness, fails an assert.
    #[test]
    fn hover_and_press_are_distinct_lifted_variants_of_panel()
    -> Result<(), ron::error::SpannedError> {
        let spec: GdtfThemeSpec = ron::from_str(SHIPPED_GRIMDARK_RON)?;
        let theme = spec.resolve(Handle::<Font>::default());

        let base = theme.panel_bg.to_srgba();
        let hover = theme.hover_bg.to_srgba();
        let press = theme.press_bg.to_srgba();

        // Both states differ from the base fill, and from each other.
        assert_ne!(
            *theme.hover_bg, *theme.panel_bg,
            "hover must differ from base"
        );
        assert_ne!(
            *theme.press_bg, *theme.panel_bg,
            "press must differ from base"
        );
        assert_ne!(
            *theme.hover_bg, *theme.press_bg,
            "hover must differ from press"
        );

        // Hover is lighter than press, and both are lifted above the base.
        // Sum the sRGB channels as a coarse "lightness" proxy.
        let lift = |c: bevy::color::Srgba| c.red + c.green + c.blue;
        assert!(
            lift(hover) > lift(press),
            "hover must be lighter than press"
        );
        assert!(lift(press) > lift(base), "press must be lifted above base");

        Ok(())
    }
}
