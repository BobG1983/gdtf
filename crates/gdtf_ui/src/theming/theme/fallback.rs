//! Built-in fallback theme when RON load fails.

use bevy::prelude::*;

use super::{
    newtypes::{
        ActiveColor, BorderColor, BorderWidthVw, ButtonColor, ContentMargin, CornerRadiusVw,
        DisabledColor, FontKey, FontSizePt, HoverColor, MarginVh, MarginVw, PanelColor,
        PressedColor, ScreenColor, TextColor,
    },
    runtime::{BackgroundTheme, ButtonTheme, GdtfTheme, PanelTheme, TextTheme, TitleTheme},
    spec::GdtfThemeSpec,
};

pub(super) const SHIPPED_GRIMDARK_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/core_tuning/ui_theme.tuning.ron"
));

/// Theme from the shipped RON, or a hard-coded grimdark fallback.
#[must_use]
pub fn default_theme() -> GdtfTheme {
    match ron::from_str::<GdtfThemeSpec>(SHIPPED_GRIMDARK_RON) {
        Ok(spec) => spec.resolve(|_| Handle::<Font>::default()),
        Err(_) => const_fallback_theme(),
    }
}

pub(super) fn const_fallback_theme() -> GdtfTheme {
    let font = Handle::<Font>::default();
    let margin = ContentMargin {
        l: MarginVw::new(0.9375),
        r: MarginVw::new(0.9375),
        t: MarginVh::new(0.83333),
        b: MarginVh::new(0.83333),
    };
    GdtfTheme {
        default_font: FontKey::new("fonts/Alegreya-Variable.ttf"),
        background: BackgroundTheme {
            color: ScreenColor::new(Color::srgba(0.05, 0.05, 0.06, 1.0)),
        },
        panel: PanelTheme {
            color: PanelColor::new(Color::srgba(0.16, 0.16, 0.18, 0.55)),
            border_color: BorderColor::new(Color::srgba(0.20, 0.20, 0.24, 1.0)),
            border_width: BorderWidthVw::new(0.15625),
            corner_radius: CornerRadiusVw::new(0.390_625),
            margin,
        },
        button: ButtonTheme {
            color: ButtonColor::new(Color::srgba(0.12, 0.12, 0.15, 0.96)),
            disabled: DisabledColor::new(Color::srgba(0.08, 0.08, 0.10, 0.55)),
            active: ActiveColor::new(Color::srgba(0.45, 0.62, 0.30, 0.96)),
            hover: HoverColor::new(Color::srgba(0.80, 0.16, 0.19, 0.96)),
            pressed: PressedColor::new(Color::srgba(0.10, 0.10, 0.12, 0.96)),
            text_color: TextColor::new(Color::srgba(0.84, 0.80, 0.73, 1.0)),
            font_size_pt: FontSizePt::new(18.0),
            border_color: BorderColor::new(Color::srgba(0.20, 0.20, 0.24, 1.0)),
            border_width: BorderWidthVw::new(0.15625),
            corner_radius: CornerRadiusVw::new(0.390_625),
            margin,
            font: font.clone(),
        },
        title: TitleTheme {
            text_color: TextColor::new(Color::srgba(0.84, 0.80, 0.73, 1.0)),
            font_size_pt: FontSizePt::new(36.0),
            font: font.clone(),
        },
        text: TextTheme {
            text_color: TextColor::new(Color::srgba(0.84, 0.80, 0.73, 1.0)),
            font_size_pt: FontSizePt::new(18.0),
            font,
        },
    }
}
