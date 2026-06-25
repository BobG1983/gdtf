//! The resolved, runtime theme: a runtime struct per widget role, the
//! [`GdtfTheme`] resource, and the [`ActiveThemeHandle`] that drives the
//! live-retheme layer.
//!
//! Every field is a typed newtype over a resolved value (a [`Color`], a
//! relative-length scalar, a loaded [`Handle<Font>`]); these structs carry
//! **no** raw `Color`/`f32`/`String` domain fields. Built only via
//! [`GdtfThemeSpec::resolve`](super::spec::GdtfThemeSpec::resolve).

use bevy::prelude::*;
use gdtf_assets::RonAsset;

use super::{
    newtypes::{
        ActiveColor, BorderColor, BorderWidthVw, ButtonColor, ContentMargin, CornerRadiusVw,
        DisabledColor, FontKey, FontSizePt, HoverColor, PanelColor, PressedColor, ScreenColor,
        TextColor,
    },
    spec::GdtfThemeSpec,
};

/// Runtime backdrop sub-theme — the full-screen fill behind all UI.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BackgroundTheme {
    /// The backdrop fill color.
    pub color: ScreenColor,
}

/// Runtime panel-box sub-theme — a box drawn around grouped UI.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PanelTheme {
    /// Panel fill color.
    pub color:         PanelColor,
    /// Panel border stroke color.
    pub border_color:  BorderColor,
    /// Panel border stroke width (`Vw`).
    pub border_width:  BorderWidthVw,
    /// Panel corner radius (`Vw`).
    pub corner_radius: CornerRadiusVw,
    /// Panel inner content padding, per edge.
    pub margin:        ContentMargin,
}

/// Runtime button sub-theme — the box, state fills, and caption typography of a
/// themed button.
#[derive(Clone, PartialEq, Debug)]
pub struct ButtonTheme {
    /// Resting button fill color.
    pub color:         ButtonColor,
    /// Disabled button fill color.
    pub disabled:      DisabledColor,
    /// Active / toggled-on button fill color (GTW-253).
    pub active:        ActiveColor,
    /// Hovered button fill color.
    pub hover:         HoverColor,
    /// Pressed button fill color.
    pub pressed:       PressedColor,
    /// Button caption text color.
    pub text_color:    TextColor,
    /// Button caption text size.
    pub font_size_pt:  FontSizePt,
    /// Button border stroke color.
    pub border_color:  BorderColor,
    /// Button border stroke width (`Vw`).
    pub border_width:  BorderWidthVw,
    /// Button corner radius (`Vw`).
    pub corner_radius: CornerRadiusVw,
    /// Button inner content padding, per edge.
    pub margin:        ContentMargin,
    /// The resolved caption font handle (the override, or the theme default).
    pub font:          Handle<Font>,
}

/// Runtime title sub-theme — the typography of a heading.
#[derive(Clone, PartialEq, Debug)]
pub struct TitleTheme {
    /// Title text color.
    pub text_color:   TextColor,
    /// Title text size.
    pub font_size_pt: FontSizePt,
    /// The resolved title font handle (the override, or the theme default).
    pub font:         Handle<Font>,
}

/// Runtime body-text sub-theme — the typography of ordinary labels / rich text.
#[derive(Clone, PartialEq, Debug)]
pub struct TextTheme {
    /// Body text color.
    pub text_color:   TextColor,
    /// Body text size.
    pub font_size_pt: FontSizePt,
    /// The resolved body-text font handle (the override, or the theme default).
    pub font:         Handle<Font>,
}

/// The resolved, runtime GDTF UI theme — a nested record of per-widget sub-themes.
///
/// Every field is a typed value (a sub-theme of newtypes over resolved
/// [`Color`]s / relative-length scalars / loaded [`Handle<Font>`]s, or the
/// [`default_font`](Self::default_font) path) — there are **no** raw
/// `Color`/`f32`/`String` domain fields. Built only via
/// [`GdtfThemeSpec::resolve`](super::spec::GdtfThemeSpec::resolve).
///
/// This resource is **not** inserted at startup. The `Load` scene populates it
/// during `AppState::Load`; readers must guard for its absence per the project's
/// state-scoped-resource convention. `Themed`/`apply_theme` consume it.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct GdtfTheme {
    /// The loose font-path key used by any text-bearing sub-theme that does not
    /// override its own font.
    pub default_font: FontKey,
    /// The full-screen backdrop sub-theme.
    pub background:   BackgroundTheme,
    /// The panel-box sub-theme.
    pub panel:        PanelTheme,
    /// The button sub-theme.
    pub button:       ButtonTheme,
    /// The title / heading sub-theme.
    pub title:        TitleTheme,
    /// The body-text sub-theme.
    pub text:         TextTheme,
}

/// The handle to the **active** theme RON asset (`theme/grimdark.ron`), held as a
/// persistent resource so the live-retheme layer can react to its changes.
///
/// A named [`Deref`] newtype over the `RonAsset<GdtfThemeSpec>` handle rather than
/// a bare `Handle` (no-bare-types rule): the name says "the theme asset currently
/// driving [`GdtfTheme`]". The retheme system
/// ([`redrive_theme_on_asset_event`](crate::theming::retheme::redrive_theme_on_asset_event))
/// filters incoming [`AssetEvent`](bevy::asset::AssetEvent) ids against this
/// handle's id, ignoring events for any other asset.
///
/// Inserted alongside [`GdtfTheme`] during `AppState::Load` (on **both** the
/// success and the const-fallback paths — the handle is valid even when the load
/// failed, so a later file-watcher reload can recover) and, like [`GdtfTheme`], it
/// **persists** past `OnExit(Load)`. Holding the handle keeps a **strong**
/// reference to the asset so it stays loaded for that future watcher.
///
/// It is **not** inserted at startup; readers guard for its absence per the
/// state-scoped-resource convention (bevy-traps rule 1).
#[derive(Resource, Deref, Clone, Debug)]
pub struct ActiveThemeHandle(Handle<RonAsset<GdtfThemeSpec>>);

impl ActiveThemeHandle {
    /// Wrap the strong [`Handle`] to the active theme RON asset.
    #[must_use]
    pub const fn new(handle: Handle<RonAsset<GdtfThemeSpec>>) -> Self {
        Self(handle)
    }
}
