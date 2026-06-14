//! Load-scoped resources for the async theme/font load orchestration.
//!
//! Per `bevy-traps.md` rule 1 there is no built-in state-scoped *resource* in
//! Bevy 0.18: [`LoadHandles`] and [`LoadFailed`] are inserted in an
//! `OnEnter(AppState::Load)` system and removed in `OnExit(AppState::Load)`, and
//! every system that reads them guards with `Option<Res<_>>` /
//! `run_if(resource_exists::<_>)`. The resolved
//! [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) is the deliberate exception that
//! *persists* past the exit, because every later scene reads it.

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use gdtf_ui::theme::GdtfThemeSpec;

/// Typed handle to the in-flight theme RON asset (`theme/grimdark.ron`).
///
/// A named newtype over the bevy [`Handle`] so the no-bare-types rule holds even
/// for asset plumbing: a bare `Handle<RonAsset<GdtfThemeSpec>>` carries no domain
/// meaning, this name says "the theme being loaded".
#[derive(Deref, Clone, Debug)]
pub(in crate::scenes::load) struct ThemeHandle(pub Handle<RonAsset<GdtfThemeSpec>>);

/// Typed handle to the in-flight theme font asset
/// (`fonts/Alegreya-Variable.ttf`).
///
/// A named newtype over the bevy [`Handle`] for the same reason as
/// [`ThemeHandle`].
#[derive(Deref, Clone, Debug)]
pub(in crate::scenes::load) struct FontHandle(pub Handle<Font>);

/// The Load-scoped handles to the assets the [`AppState::Load`](crate::states::AppState::Load)
/// kick-off started loading.
///
/// Holds the typed [`ThemeHandle`] and [`FontHandle`] the poll/resolve system
/// reads each frame to check load progress. Inserted `OnEnter(Load)` and removed
/// `OnExit(Load)` (it has no meaning outside `Load`).
#[derive(Resource, Clone, Debug)]
pub(in crate::scenes::load) struct LoadHandles {
    /// The theme RON asset being loaded.
    pub theme: ThemeHandle,
    /// The theme font asset being loaded.
    pub font:  FontHandle,
}

/// The loose-asset path of a load that reached
/// [`LoadState::Failed`](bevy::asset::LoadState::Failed).
///
/// A named newtype over the path `String` so the failure record names *what*
/// failed in the type, not a bare string.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub(in crate::scenes::load) struct FailedAssetPath(pub String);

/// Records that a required `Load` asset failed to load.
///
/// Inserted by the poll/resolve system when the theme RON (or its font) reaches
/// [`LoadState::Failed`](bevy::asset::LoadState::Failed); the system then warns
/// naming the failed path and inserts the const-fallback
/// [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) so the app never hangs and never
/// leaves `Load` themeless. Load-scoped: removed `OnExit(Load)`.
#[derive(Resource, Deref, Clone, PartialEq, Eq, Debug)]
pub(in crate::scenes::load) struct LoadFailed(pub FailedAssetPath);
