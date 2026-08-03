//! Runtime theme resource and section types.

use bevy::prelude::*;

use super::newtypes::{
    ActiveColor, BorderColor, BorderWidthVw, ButtonColor, ContentMargin, CornerRadiusVw,
    DisabledColor, FontKey, FontSizePt, HoverColor, PanelColor, PressedColor, ScreenColor,
    TextColor,
};

/// Screen background section.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BackgroundTheme {
    /// Background fill.
    pub color: ScreenColor,
}

/// Panel chrome section.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PanelTheme {
    /// Panel fill.
    pub color: PanelColor,
    /// Border color.
    pub border_color: BorderColor,
    /// Border width.
    pub border_width: BorderWidthVw,
    /// Corner radius.
    pub corner_radius: CornerRadiusVw,
    /// Content margin.
    pub margin: ContentMargin,
}

/// Button chrome and text section.
#[derive(Clone, PartialEq, Debug)]
pub struct ButtonTheme {
    /// Default fill.
    pub color: ButtonColor,
    /// Disabled fill.
    pub disabled: DisabledColor,
    /// Active/selected fill.
    pub active: ActiveColor,
    /// Hover fill.
    pub hover: HoverColor,
    /// Pressed fill.
    pub pressed: PressedColor,
    /// Caption color.
    pub text_color: TextColor,
    /// Caption size.
    pub font_size_pt: FontSizePt,
    /// Border color.
    pub border_color: BorderColor,
    /// Border width.
    pub border_width: BorderWidthVw,
    /// Corner radius.
    pub corner_radius: CornerRadiusVw,
    /// Content margin.
    pub margin: ContentMargin,
    /// Caption font handle.
    pub font: Handle<Font>,
}

/// Title text section.
#[derive(Clone, PartialEq, Debug)]
pub struct TitleTheme {
    /// Title color.
    pub text_color: TextColor,
    /// Title size.
    pub font_size_pt: FontSizePt,
    /// Title font handle.
    pub font: Handle<Font>,
}

/// Body text section.
#[derive(Clone, PartialEq, Debug)]
pub struct TextTheme {
    /// Body color.
    pub text_color: TextColor,
    /// Body size.
    pub font_size_pt: FontSizePt,
    /// Body font handle.
    pub font: Handle<Font>,
}

/// Full runtime UI theme resource.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct GdtfTheme {
    /// Default font asset key.
    pub default_font: FontKey,
    /// Background section.
    pub background: BackgroundTheme,
    /// Panel section.
    pub panel: PanelTheme,
    /// Button section.
    pub button: ButtonTheme,
    /// Title section.
    pub title: TitleTheme,
    /// Body text section.
    pub text: TextTheme,
}
