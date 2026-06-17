//! Data-driven UI theme: the on-disk RON shape, its typed runtime form, and the
//! pure spec-to-resource resolution that bridges them.
//!
//! The theme is **data-driven** (ADR 0003): visual constants live in a loose
//! `assets/theme/*.ron` file, not in code. As of GTW-149 the theme is **nested**
//! into per-widget sub-themes rather than one flat record: a top-level
//! [`GdtfTheme`] holds a [`BackgroundTheme`], a [`PanelTheme`], a [`ButtonTheme`],
//! a [`TitleTheme`], and a [`TextTheme`], each painting one role of the UI.
//!
//! This module owns three layers, keeping the spec/runtime split per sub-theme:
//!
//! - [`GdtfThemeSpec`] (and a `*Spec` for each sub-theme) — the serde-`Deserialize`
//!   mirror of the on-disk RON. It holds wire-friendly shapes ([`Srgba4`] color
//!   quads, `String` font keys, bare px scalars) and nothing Bevy-asset-bound, so
//!   it deserializes with no `World`.
//! - [`GdtfTheme`] (and a runtime struct for each sub-theme) — the runtime
//!   `Resource`. Every field is a typed newtype over a resolved value (a
//!   `Color`, a px scalar, a loaded `Handle<Font>`); it carries **no** raw
//!   `Color`/`f32`/`String` domain fields.
//! - [`GdtfThemeSpec::resolve`] — the pure bridge: it consumes a spec plus a
//!   **font resolver** closure and produces a [`GdtfTheme`]. It touches no `World`
//!   and no `AssetServer` directly (the resolver abstracts that), so it is
//!   unit-testable fully headless.
//!
//! ## Font overrides
//!
//! Only the **text-bearing** sub-themes ([`ButtonTheme`], [`TitleTheme`],
//! [`TextTheme`]) carry an optional per-sub-theme `font` override: in the spec it
//! is an `Option<String>` with `#[serde(default)]`, so an absent field means
//! "use the theme's [`default_font`](GdtfThemeSpec::default_font)". The
//! [`BackgroundTheme`] and [`PanelTheme`] render no text, so they carry **no**
//! font field at all. Resolution picks `font.as_deref().unwrap_or(&default_font)`
//! and feeds that key through the resolver, so each text-bearing runtime sub-theme
//! ends up with a concrete resolved `Handle<Font>`.
//!
//! The module is split by layer: `newtypes` (the typed leaves + wire-shaped
//! `Srgba4`/`MarginSpec`), `spec` (the `*Spec` mirror + resolution), `runtime`
//! (the resolved sub-themes + [`GdtfTheme`]/[`ActiveThemeHandle`]), and `fallback`
//! (the error-path [`default_theme`]).

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
    ActiveColor, BorderColor, BorderWidthPx, ButtonColor, ContentMargin, CornerRadiusPx,
    DisabledColor, FontKey, FontSizePt, HoverColor, MarginPx, MarginSpec, PanelColor, PressedColor,
    ScreenColor, Srgba4, TextColor,
};
pub use runtime::{
    ActiveThemeHandle, BackgroundTheme, ButtonTheme, GdtfTheme, PanelTheme, TextTheme, TitleTheme,
};
pub use spec::{
    BackgroundThemeSpec, ButtonThemeSpec, GdtfThemeSpec, PanelThemeSpec, TextThemeSpec,
    TitleThemeSpec,
};
