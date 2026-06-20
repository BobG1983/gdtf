//! Load-scoped resources for the async theme/font load orchestration.
//!
//! Per `bevy-traps.md` rule 1 there is no built-in state-scoped *resource* in
//! Bevy 0.18: [`LoadHandles`] and [`LoadFailed`] are inserted in an
//! `OnEnter(AppState::Load)` system and removed in `OnExit(AppState::Load)`, and
//! every system that reads them guards with `Option<Res<_>>` /
//! `run_if(resource_exists::<_>)`. The resolved
//! [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) is the deliberate exception that
//! *persists* past the exit, because every later scene reads it.

use bevy::{asset::LoadedFolder, prelude::*};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{situation::Situation, tuning::CombatTuning};
use gdtf_ui::theme::GdtfThemeSpec;

/// Typed handle to the in-flight theme RON asset (`theme/grimdark.ron`).
///
/// A named newtype over the bevy [`Handle`] so the no-bare-types rule holds even
/// for asset plumbing: a bare `Handle<RonAsset<GdtfThemeSpec>>` carries no domain
/// meaning, this name says "the theme being loaded".
#[derive(Deref, Clone, Debug)]
pub(in crate::scenes::load) struct ThemeHandle(Handle<RonAsset<GdtfThemeSpec>>);

impl ThemeHandle {
    /// Wrap an in-flight theme RON asset handle.
    pub(in crate::scenes::load) const fn new(handle: Handle<RonAsset<GdtfThemeSpec>>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **fonts folder** load (`fonts/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule).
/// As of GTW-149 the `Load` scene preloads ALL fonts up front by loading the
/// `fonts` folder rather than a single font: holding this handle keeps a strong
/// reference to every font in the folder so any font a theme (or a hot-reloaded
/// theme) selects is already resident and `asset_server.load(key)` returns the
/// loaded handle idempotently.
#[derive(Deref, Clone, Debug)]
pub(in crate::scenes::load) struct FontFolderHandle(Handle<LoadedFolder>);

impl FontFolderHandle {
    /// Wrap an in-flight fonts-folder load handle.
    pub(in crate::scenes::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **weapons folder** load (`weapons/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`FontFolderHandle`] (GTW-257). The `Load` scene preloads the whole
/// `assets/weapons/` folder up front via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder); the
/// poll/resolve system gates on its recursive load state, then builds the
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) from the loaded
/// `RonAsset<WeaponSpec>` files (keyed by filename stem). Holding this handle keeps
/// a strong reference to every weapon asset while the registry is built; the
/// registry then holds the specs BY VALUE, so they survive the handle being dropped
/// on `OnExit(Load)`.
#[derive(Deref, Clone, Debug)]
pub(in crate::scenes::load) struct WeaponsFolderHandle(Handle<LoadedFolder>);

impl WeaponsFolderHandle {
    /// Wrap an in-flight weapons-folder load handle.
    pub(in crate::scenes::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **armor folder** load (`armor/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`WeaponsFolderHandle`] (GTW-269). The `Load` scene preloads the whole
/// `assets/armor/` folder up front via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder); the
/// poll/resolve system gates on its recursive load state, then builds the
/// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry) from the loaded
/// `RonAsset<ArmorSpec>` files (keyed by filename stem). Holding this handle keeps a
/// strong reference to every armor asset while the registry is built; the registry
/// then holds the specs BY VALUE, so they survive the handle being dropped on
/// `OnExit(Load)`.
#[derive(Deref, Clone, Debug)]
pub(in crate::scenes::load) struct ArmorsFolderHandle(Handle<LoadedFolder>);

impl ArmorsFolderHandle {
    /// Wrap an in-flight armor-folder load handle.
    pub(in crate::scenes::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight situation RON asset (`situations/skirmish.ron`).
///
/// A named newtype over the bevy [`Handle`] so the no-bare-types rule holds even
/// for asset plumbing: a bare `Handle<RonAsset<Situation>>` carries no domain
/// meaning, this name says "the authored battlefield being loaded" (GTW-205 /
/// E10.3). The poll/resolve system reads it to check the load's progress, then
/// resolves it into the persistent [`LoadedSituation`].
#[derive(Deref, Clone, Debug)]
pub(in crate::scenes::load) struct SituationHandle(Handle<RonAsset<Situation>>);

impl SituationHandle {
    /// Wrap an in-flight situation RON asset handle.
    pub(in crate::scenes::load) const fn new(handle: Handle<RonAsset<Situation>>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight combat-tuning RON asset (`combat/tuning.ron`).
///
/// A named newtype over the bevy [`Handle`] so the no-bare-types rule holds even
/// for asset plumbing: a bare `Handle<RonAsset<CombatTuning>>` carries no domain
/// meaning, this name says "the shipped combat tuning being loaded" (GTW-206 /
/// E10.4). The poll/resolve system reads it to check the load's progress, then
/// inserts the deserialized [`CombatTuning`] as the persistent runtime resource.
#[derive(Deref, Clone, Debug)]
pub(in crate::scenes::load) struct TuningHandle(Handle<RonAsset<CombatTuning>>);

impl TuningHandle {
    /// Wrap an in-flight combat-tuning RON asset handle.
    pub(in crate::scenes::load) const fn new(handle: Handle<RonAsset<CombatTuning>>) -> Self {
        Self(handle)
    }
}

/// The Load-scoped handles to the assets the [`AppState::Load`](crate::states::AppState::Load)
/// kick-off started loading.
///
/// Holds the typed [`ThemeHandle`], [`FontFolderHandle`], [`SituationHandle`],
/// [`TuningHandle`], [`WeaponsFolderHandle`], and [`ArmorsFolderHandle`] the
/// poll/resolve system reads each frame to check load progress. Inserted
/// `OnEnter(Load)` and removed `OnExit(Load)` (it has no meaning outside `Load`).
#[derive(Resource, Clone, Debug)]
pub(in crate::scenes::load) struct LoadHandles {
    /// The theme RON asset being loaded.
    pub theme:     ThemeHandle,
    /// The fonts folder being preloaded (all fonts up front).
    pub fonts:     FontFolderHandle,
    /// The authored situation RON asset being loaded (GTW-205 / E10.3).
    pub situation: SituationHandle,
    /// The shipped combat-tuning RON asset being loaded (GTW-206 / E10.4).
    pub tuning:    TuningHandle,
    /// The weapons folder being preloaded (all weapon `.ron`s up front, GTW-257).
    pub weapons:   WeaponsFolderHandle,
    /// The armor folder being preloaded (all armor `.ron`s up front, GTW-269).
    pub armor:     ArmorsFolderHandle,
}

crate::support_item! {
    /// The resolved authored battlefield — the persistent [`Situation`] resource the
    /// later Generation slice (E10.5) reads on `BattleScapeState::Generation`.
    ///
    /// A named newtype over [`Situation`] (no-bare-types) that [`Deref`]s to it, so a
    /// consumer reads the situation's fields straight through. The poll/resolve system
    /// inserts it once the loaded `RonAsset<Situation>` resolves, and — like
    /// [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) and
    /// [`ActiveThemeHandle`](gdtf_ui::theme::ActiveThemeHandle) — it is the deliberate
    /// exception that **persists** past `OnExit(Load)` (it is **not** removed in
    /// `cleanup`), so the authored battlefield outlives `Load` for the Generation
    /// consumer (bevy-traps rule 1: a state-scoped-exception resource).
    ///
    /// Declared through [`crate::support_item!`] so it is `pub` under `test-support`
    /// (the AC7 real-asset harness names it) and `pub(crate)` in the binary build
    /// (E10.5 consumes it in-crate) — keeping the binary `unreachable_pub`-clean.
    #[derive(Resource, Deref, Clone, Debug)]
    struct LoadedSituation(Situation);
}

impl LoadedSituation {
    // `new` is `pub` under `test-support` (the AC7 real-asset harness + the
    // `gdtf_test_utils` battle builder construct it) and `pub(crate)` in the
    // binary, via `support_item!` per method — the same visibility flip the type
    // itself uses, so `unreachable_pub` stays satisfied in both configurations.
    crate::support_item! {
        /// Wrap a resolved authored battlefield as the persistent resource.
        const fn new(situation: Situation) -> Self {
            Self(situation)
        }
    }
}

/// The loose-asset path of a load that reached
/// [`LoadState::Failed`](bevy::asset::LoadState::Failed).
///
/// A named newtype over the path `String` so the failure record names *what*
/// failed in the type, not a bare string.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub(in crate::scenes::load) struct FailedAssetPath(String);

impl FailedAssetPath {
    /// Record the loose-asset path of a load that failed.
    pub(in crate::scenes::load) fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
}

/// Records that a required `Load` asset failed to load.
///
/// Inserted by the poll/resolve system when the theme RON (or its font) reaches
/// [`LoadState::Failed`](bevy::asset::LoadState::Failed); the system then warns
/// naming the failed path and inserts the const-fallback
/// [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) so the app never hangs and never
/// leaves `Load` themeless. Load-scoped: removed `OnExit(Load)`.
#[derive(Resource, Deref, Clone, PartialEq, Eq, Debug)]
pub(in crate::scenes::load) struct LoadFailed(FailedAssetPath);

impl LoadFailed {
    /// Record that a required `Load` asset failed to load, naming its path.
    pub(in crate::scenes::load) const fn new(path: FailedAssetPath) -> Self {
        Self(path)
    }
}
