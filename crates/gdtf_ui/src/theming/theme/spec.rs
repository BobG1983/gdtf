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

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct BackgroundThemeSpec {
        pub color: Srgba4,
}

impl BackgroundThemeSpec {
        pub(super) const fn resolve(self) -> BackgroundTheme {
        BackgroundTheme {
            color: ScreenColor::new(self.color.into_color()),
        }
    }
}

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct PanelThemeSpec {
        pub color:         Srgba4,
        pub border_color:  Srgba4,
        pub border_width:  f32,
        pub corner_radius: f32,
        pub margin:        MarginSpec,
}

impl PanelThemeSpec {
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

/// (`#[serde(default)]` — absent means "use the theme's `default_font`").
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct ButtonThemeSpec {
        pub color:         Srgba4,
        pub disabled:      Srgba4,
        pub active:        Srgba4,
        pub hover:         Srgba4,
        pub pressed:       Srgba4,
        pub text_color:    Srgba4,
        pub font_size_pt:  f32,
        pub border_color:  Srgba4,
        pub border_width:  f32,
        pub corner_radius: f32,
        pub margin:        MarginSpec,
        #[serde(default)]
    pub font:          Option<String>,
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

/// (`#[serde(default)]` — absent means "use the theme's `default_font`").
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct TitleThemeSpec {
        pub text_color:   Srgba4,
        pub font_size_pt: f32,
        #[serde(default)]
    pub font:         Option<String>,
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

/// (`#[serde(default)]` — absent means "use the theme's `default_font`").
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct TextThemeSpec {
        pub text_color:   Srgba4,
        pub font_size_pt: f32,
        #[serde(default)]
    pub font:         Option<String>,
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

#[derive(Deserialize, TypePath, Clone, PartialEq, Debug)]
pub struct GdtfThemeSpec {
        pub default_font: String,
        pub background:   BackgroundThemeSpec,
        pub panel:        PanelThemeSpec,
        pub button:       ButtonThemeSpec,
        pub title:        TitleThemeSpec,
        pub text:         TextThemeSpec,
}

impl GdtfThemeSpec {
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
