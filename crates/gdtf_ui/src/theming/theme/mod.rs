//! is an `Option<String>` with `#[serde(default)]`, so an absent field means
mod fallback;
mod newtypes;
mod runtime;
mod spec;
#[cfg(test)]
mod test_fallback;
#[cfg(test)]
mod test_resolve;

pub use fallback::default_theme;
pub use newtypes::{
    ActiveColor, BorderColor, BorderWidthVw, ButtonColor, ContentMargin, CornerRadiusVw,
    DisabledColor, FontKey, FontSizePt, HoverColor, MarginSpec, MarginVh, MarginVw, PanelColor,
    PressedColor, ScreenColor, Srgba4, TextColor,
};
pub use runtime::{BackgroundTheme, ButtonTheme, GdtfTheme, PanelTheme, TextTheme, TitleTheme};
pub use spec::{
    BackgroundThemeSpec, ButtonThemeSpec, GdtfThemeSpec, PanelThemeSpec, TextThemeSpec,
    TitleThemeSpec,
};
