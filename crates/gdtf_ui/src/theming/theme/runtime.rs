use bevy::prelude::*;

use super::newtypes::{
    ActiveColor, BorderColor, BorderWidthVw, ButtonColor, ContentMargin, CornerRadiusVw,
    DisabledColor, FontKey, FontSizePt, HoverColor, PanelColor, PressedColor, ScreenColor,
    TextColor,
};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BackgroundTheme {
        pub color: ScreenColor,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PanelTheme {
        pub color:         PanelColor,
        pub border_color:  BorderColor,
        pub border_width:  BorderWidthVw,
        pub corner_radius: CornerRadiusVw,
        pub margin:        ContentMargin,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ButtonTheme {
        pub color:         ButtonColor,
        pub disabled:      DisabledColor,
        pub active:        ActiveColor,
        pub hover:         HoverColor,
        pub pressed:       PressedColor,
        pub text_color:    TextColor,
        pub font_size_pt:  FontSizePt,
        pub border_color:  BorderColor,
        pub border_width:  BorderWidthVw,
        pub corner_radius: CornerRadiusVw,
        pub margin:        ContentMargin,
        pub font:          Handle<Font>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TitleTheme {
        pub text_color:   TextColor,
        pub font_size_pt: FontSizePt,
        pub font:         Handle<Font>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TextTheme {
        pub text_color:   TextColor,
        pub font_size_pt: FontSizePt,
        pub font:         Handle<Font>,
}

#[derive(Resource, Clone, PartialEq, Debug)]
pub struct GdtfTheme {
            pub default_font: FontKey,
        pub background:   BackgroundTheme,
        pub panel:        PanelTheme,
        pub button:       ButtonTheme,
        pub title:        TitleTheme,
        pub text:         TextTheme,
}
