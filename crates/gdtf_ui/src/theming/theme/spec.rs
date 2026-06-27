//! The on-disk RON mirror: a `*Spec` per widget role plus the top-level
//! [`GdtfThemeSpec`], and the pure spec→runtime resolution.
//!
//! These types hold wire-friendly shapes ([`Srgba4`] color quads, `String` font
//! keys, bare relative-length scalars) and nothing Bevy-asset-bound, so they
//! deserialize with no `World`. Each carries a `resolve` that maps it into the
//! matching runtime sub-theme of [`runtime`](super::runtime).

use bevy::prelude::*;
use serde::Deserialize;

use super::{
    newtypes::{
        ActiveColor, BorderColor, BorderWidthVw, ButtonColor, CornerRadiusVw, DisabledColor,
        FontKey, FontSizePt, HoverColor, MarginSpec, PanelColor, PressedColor, ScreenColor, Srgba4,
        TextColor,
    },
    runtime::{BackgroundTheme, ButtonTheme, GdtfTheme, PanelTheme, TextTheme, TitleTheme},
};

/// On-disk shape of the full-screen backdrop sub-theme.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct BackgroundThemeSpec {
    /// The backdrop fill color.
    pub color: Srgba4,
}

impl BackgroundThemeSpec {
    /// Resolve the backdrop sub-theme (no font, no resolver needed).
    pub(super) const fn resolve(self) -> BackgroundTheme {
        BackgroundTheme {
            color: ScreenColor::new(self.color.into_color()),
        }
    }
}

/// On-disk shape of the panel-box sub-theme.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct PanelThemeSpec {
    /// Panel fill color.
    pub color:         Srgba4,
    /// Panel border stroke color.
    pub border_color:  Srgba4,
    /// Panel border stroke width, as a window-WIDTH fraction (`Vw`).
    pub border_width:  f32,
    /// Panel corner radius, as a window-WIDTH fraction (`Vw`).
    pub corner_radius: f32,
    /// Panel inner content padding, per edge.
    pub margin:        MarginSpec,
}

impl PanelThemeSpec {
    /// Resolve the panel sub-theme (no font, no resolver needed).
    pub(super) const fn resolve(self) -> PanelTheme {
        PanelTheme {
            color:         PanelColor::new(self.color.into_color()),
            border_color:  BorderColor::new(self.border_color.into_color()),
            border_width:  BorderWidthVw::new(self.border_width),
            corner_radius: CornerRadiusVw::new(self.corner_radius),
            margin:        self.margin.resolve(),
        }
    }
}

/// On-disk shape of the button sub-theme.
///
/// Text-bearing, so it carries an optional [`font`](Self::font) override
/// (`#[serde(default)]` — absent means "use the theme's `default_font`").
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct ButtonThemeSpec {
    /// Resting button fill color.
    pub color:         Srgba4,
    /// Disabled button fill color.
    pub disabled:      Srgba4,
    /// Active / toggled-on button fill color (GTW-253).
    pub active:        Srgba4,
    /// Hovered button fill color.
    pub hover:         Srgba4,
    /// Pressed button fill color.
    pub pressed:       Srgba4,
    /// Button caption text color.
    pub text_color:    Srgba4,
    /// Button caption text size, in typographic points.
    pub font_size_pt:  f32,
    /// Button border stroke color.
    pub border_color:  Srgba4,
    /// Button border stroke width, as a window-WIDTH fraction (`Vw`).
    pub border_width:  f32,
    /// Button corner radius, as a window-WIDTH fraction (`Vw`).
    pub corner_radius: f32,
    /// Button inner content padding, per edge.
    pub margin:        MarginSpec,
    /// Optional font override; absent means use the theme `default_font`.
    #[serde(default)]
    pub font:          Option<String>,
}

impl ButtonThemeSpec {
    /// Resolve the button sub-theme, threading the chosen font handle in.
    fn resolve(self, font: Handle<Font>) -> ButtonTheme {
        ButtonTheme {
            color: ButtonColor::new(self.color.into_color()),
            disabled: DisabledColor::new(self.disabled.into_color()),
            active: ActiveColor::new(self.active.into_color()),
            hover: HoverColor::new(self.hover.into_color()),
            pressed: PressedColor::new(self.pressed.into_color()),
            text_color: TextColor::new(self.text_color.into_color()),
            font_size_pt: FontSizePt::new(self.font_size_pt),
            border_color: BorderColor::new(self.border_color.into_color()),
            border_width: BorderWidthVw::new(self.border_width),
            corner_radius: CornerRadiusVw::new(self.corner_radius),
            margin: self.margin.resolve(),
            font,
        }
    }
}

/// On-disk shape of the title sub-theme.
///
/// Text-bearing, so it carries an optional [`font`](Self::font) override
/// (`#[serde(default)]` — absent means "use the theme's `default_font`").
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct TitleThemeSpec {
    /// Title text color.
    pub text_color:   Srgba4,
    /// Title text size, in typographic points.
    pub font_size_pt: f32,
    /// Optional font override; absent means use the theme `default_font`.
    #[serde(default)]
    pub font:         Option<String>,
}

impl TitleThemeSpec {
    /// Resolve the title sub-theme, threading the chosen font handle in.
    fn resolve(self, font: Handle<Font>) -> TitleTheme {
        TitleTheme {
            text_color: TextColor::new(self.text_color.into_color()),
            font_size_pt: FontSizePt::new(self.font_size_pt),
            font,
        }
    }
}

/// On-disk shape of the body-text sub-theme.
///
/// Text-bearing, so it carries an optional [`font`](Self::font) override
/// (`#[serde(default)]` — absent means "use the theme's `default_font`").
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct TextThemeSpec {
    /// Body text color.
    pub text_color:   Srgba4,
    /// Body text size, in typographic points.
    pub font_size_pt: f32,
    /// Optional font override; absent means use the theme `default_font`.
    #[serde(default)]
    pub font:         Option<String>,
}

impl TextThemeSpec {
    /// Resolve the body-text sub-theme, threading the chosen font handle in.
    fn resolve(self, font: Handle<Font>) -> TextTheme {
        TextTheme {
            text_color: TextColor::new(self.text_color.into_color()),
            font_size_pt: FontSizePt::new(self.font_size_pt),
            font,
        }
    }
}

/// The on-disk RON shape of a GDTF theme.
///
/// The deserialization mirror of `assets/theme/*.ron`: a `default_font` path plus
/// one nested `*Spec` per widget role. It carries nothing Bevy-asset-bound, so
/// `ron::from_str` into it needs no `World` and no `AssetServer`.
///
/// Resolve it into the runtime [`GdtfTheme`] with [`GdtfThemeSpec::resolve`].
///
/// It derives [`TypePath`] so it can be the payload of a
/// `RonAsset<GdtfThemeSpec>` (the generic GTW-136 loader requires `T: TypePath`):
/// the `Load` scene loads `core_tuning/ui_theme.tuning.ron` as that asset, then resolves the
/// deserialized spec into a [`GdtfTheme`].
#[derive(Deserialize, TypePath, Clone, PartialEq, Debug)]
pub struct GdtfThemeSpec {
    /// The font used by any text-bearing sub-theme that does not override it.
    pub default_font: String,
    /// The full-screen backdrop sub-theme.
    pub background:   BackgroundThemeSpec,
    /// The panel-box sub-theme.
    pub panel:        PanelThemeSpec,
    /// The button sub-theme.
    pub button:       ButtonThemeSpec,
    /// The title / heading sub-theme.
    pub title:        TitleThemeSpec,
    /// The body-text sub-theme.
    pub text:         TextThemeSpec,
}

impl GdtfThemeSpec {
    /// Resolve this on-disk spec into the runtime [`GdtfTheme`] resource.
    ///
    /// Pure: it consumes the spec and a `resolve_font` closure that maps a loose
    /// font-path key to a [`Handle<Font>`], and builds the typed runtime values.
    /// It accesses no `World` and no `AssetServer` itself — the closure abstracts
    /// font loading — so it is fully unit-testable headless. For each text-bearing
    /// sub-theme it picks the sub-theme's `font` override if present, else the
    /// top-level [`default_font`](Self::default_font), and resolves that key.
    ///
    /// In the running app the closure is `|key| asset_server.load::<Font>(key)`
    /// (idempotent — it returns the already-loaded handle for a preloaded font);
    /// in the error-path fallback it returns `Handle::<Font>::default()`.
    #[must_use]
    pub fn resolve(self, resolve_font: impl Fn(&str) -> Handle<Font>) -> GdtfTheme {
        let button_font = resolve_font(self.button.font.as_deref().unwrap_or(&self.default_font));
        let title_font = resolve_font(self.title.font.as_deref().unwrap_or(&self.default_font));
        let text_font = resolve_font(self.text.font.as_deref().unwrap_or(&self.default_font));

        GdtfTheme {
            default_font: FontKey::new(self.default_font),
            background:   self.background.resolve(),
            panel:        self.panel.resolve(),
            button:       self.button.resolve(button_font),
            title:        self.title.resolve(title_font),
            text:         self.text.resolve(text_font),
        }
    }
}
