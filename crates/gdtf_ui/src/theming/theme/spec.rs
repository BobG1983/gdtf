//! RON-deserializable theme specs that resolve into runtime themes.

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

/// Background section as stored in RON.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct BackgroundThemeSpec {
    /// Screen color channels.
    pub color: Srgba4,
}

impl BackgroundThemeSpec {
    pub(super) const fn resolve(self) -> BackgroundTheme {
        BackgroundTheme {
            color: ScreenColor::new(self.color.into_color()),
        }
    }
}

/// Panel section as stored in RON.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct PanelThemeSpec {
    /// Fill color.
    pub color: Srgba4,
    /// Border color.
    pub border_color: Srgba4,
    /// Border width (vw).
    pub border_width: f32,
    /// Corner radius (vw).
    pub corner_radius: f32,
    /// Content margin.
    pub margin: MarginSpec,
}

impl PanelThemeSpec {
    pub(super) const fn resolve(self) -> PanelTheme {
        PanelTheme {
            color: PanelColor::new(self.color.into_color()),
            border_color: BorderColor::new(self.border_color.into_color()),
            border_width: BorderWidthVw::new(self.border_width),
            corner_radius: CornerRadiusVw::new(self.corner_radius),
            margin: self.margin.resolve(),
        }
    }
}

/// Button section as stored in RON.
///
/// `font` is optional (`#[serde(default)]`); absent means use the theme's `default_font`.
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct ButtonThemeSpec {
    /// Default fill.
    pub color: Srgba4,
    /// Disabled fill.
    pub disabled: Srgba4,
    /// Active fill.
    pub active: Srgba4,
    /// Hover fill.
    pub hover: Srgba4,
    /// Pressed fill.
    pub pressed: Srgba4,
    /// Caption color.
    pub text_color: Srgba4,
    /// Caption size (pt).
    pub font_size_pt: f32,
    /// Border color.
    pub border_color: Srgba4,
    /// Border width (vw).
    pub border_width: f32,
    /// Corner radius (vw).
    pub corner_radius: f32,
    /// Content margin.
    pub margin: MarginSpec,
    /// Optional font asset key.
    #[serde(default)]
    pub font: Option<String>,
}

impl ButtonThemeSpec {
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

/// Title section as stored in RON.
///
/// `font` is optional; absent means use the theme's `default_font`.
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct TitleThemeSpec {
    /// Title color.
    pub text_color: Srgba4,
    /// Title size (pt).
    pub font_size_pt: f32,
    /// Optional font asset key.
    #[serde(default)]
    pub font: Option<String>,
}

impl TitleThemeSpec {
    fn resolve(self, font: Handle<Font>) -> TitleTheme {
        TitleTheme {
            text_color: TextColor::new(self.text_color.into_color()),
            font_size_pt: FontSizePt::new(self.font_size_pt),
            font,
        }
    }
}

/// Body text section as stored in RON.
///
/// `font` is optional; absent means use the theme's `default_font`.
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct TextThemeSpec {
    /// Body color.
    pub text_color: Srgba4,
    /// Body size (pt).
    pub font_size_pt: f32,
    /// Optional font asset key.
    #[serde(default)]
    pub font: Option<String>,
}

impl TextThemeSpec {
    fn resolve(self, font: Handle<Font>) -> TextTheme {
        TextTheme {
            text_color: TextColor::new(self.text_color.into_color()),
            font_size_pt: FontSizePt::new(self.font_size_pt),
            font,
        }
    }
}

/// Full theme as stored in RON.
#[derive(Deserialize, TypePath, Clone, PartialEq, Debug)]
pub struct GdtfThemeSpec {
    /// Default font asset key.
    pub default_font: String,
    /// Background section.
    pub background: BackgroundThemeSpec,
    /// Panel section.
    pub panel: PanelThemeSpec,
    /// Button section.
    pub button: ButtonThemeSpec,
    /// Title section.
    pub title: TitleThemeSpec,
    /// Body text section.
    pub text: TextThemeSpec,
}

impl GdtfThemeSpec {
    /// Resolve into a runtime theme, loading fonts via `resolve_font`.
    #[must_use]
    pub fn resolve(self, resolve_font: impl Fn(&str) -> Handle<Font>) -> GdtfTheme {
        let button_font = resolve_font(self.button.font.as_deref().unwrap_or(&self.default_font));
        let title_font = resolve_font(self.title.font.as_deref().unwrap_or(&self.default_font));
        let text_font = resolve_font(self.text.font.as_deref().unwrap_or(&self.default_font));

        GdtfTheme {
            default_font: FontKey::new(self.default_font),
            background: self.background.resolve(),
            panel: self.panel.resolve(),
            button: self.button.resolve(button_font),
            title: self.title.resolve(title_font),
            text: self.text.resolve(text_font),
        }
    }
}
