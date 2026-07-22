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
use gdtf_assets::HotRonHandle;
use gdtf_battle_sim::situation::Situation;
use gdtf_ui::theme::GdtfThemeSpec;

/// Typed handle to the in-flight **fonts folder** load (`fonts/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule).
/// As of GTW-149 the `Load` scene preloads ALL fonts up front by loading the
/// `fonts` folder rather than a single font: holding this handle keeps a strong
/// reference to every font in the folder so any font a theme (or a hot-reloaded
/// theme) selects is already resident and `asset_server.load(key)` returns the
/// loaded handle idempotently.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct FontFolderHandle(Handle<LoadedFolder>);

impl FontFolderHandle {
    /// Wrap an in-flight fonts-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **injuries folder** load (`injuries/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`FontFolderHandle`] (GTW-437). The `Load` scene preloads the whole
/// `assets/content/injuries/` folder up front via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder) (recursive,
/// so the per-part subfolders + the `weighting/` subfolder are all covered); the
/// poll/resolve system gates on its recursive load state, then builds BOTH the
/// [`InjuryRegistry`](gdtf_battle_sim::injuries::InjuryRegistry) (from the loaded
/// `RonAsset<InjuryDef>` files, keyed by stem) and the
/// [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables) (from the loaded
/// `RonAsset<InjuryWeighting>` files). Holding this handle keeps a strong reference to
/// every injury asset while the resources are built; they then hold their data BY VALUE,
/// so they survive the handle being dropped on `OnExit(Load)`.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct InjuriesFolderHandle(Handle<LoadedFolder>);

impl InjuriesFolderHandle {
    /// Wrap an in-flight injuries-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **maps (prefab) folder** load (`maps/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`InjuriesFolderHandle`] (GTW-489 — child T05c of the GTW-476 data-model
/// refactor). The UUID-keyed [`PrefabSpec`](gdtf_battle_sim::level::PrefabSpec)
/// fragments (GTW-486) live under the nested `assets/content/maps/<theme>/<size>/` tree.
/// Each fragment carries the dedicated `prefab.ron` compound extension (de-versioned
/// in GTW-557) so the recursive `load_folder` dispatches it to the
/// [`RonAsset<PrefabSpec>`](gdtf_assets::RonAsset) loader. The poll/resolve system gates
/// on this folder's recursive load state, then builds the
/// [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry) from the loaded
/// `RonAsset<PrefabSpec>` members (bucketed by each spec's `(theme, size, role)`, with NO
/// edge-opening validation — the schema has none). Holding this handle keeps a strong
/// reference to every prefab asset while the registry is built; the registry then holds
/// the prefabs BY VALUE, so they survive the handle being dropped on `OnExit(Load)`.
///
/// **GTW-494** — this is the ONLY prefab loader in the Load flow: the legacy flat-dir
/// `content/maps/*.prefab.ron` loader was retired (the procgen pipeline consumes
/// [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry) as of GTW-492).
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct PrefabsFolderHandle(Handle<LoadedFolder>);

impl PrefabsFolderHandle {
    /// Wrap an in-flight maps (prefab) folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The Load-scoped handles to the assets the [`AppState::Load`](crate::states::AppState::Load)
/// kick-off started loading.
///
/// Holds the theme's generic [`HotRonHandle`] plus the typed [`FontFolderHandle`],
/// [`InjuriesFolderHandle`], and [`PrefabsFolderHandle`]
/// the poll/resolve system reads each frame to check load progress. Inserted
/// `OnEnter(Load)` and removed `OnExit(Load)` (it has no meaning outside `Load`).
///
/// GTW-564 moved the four single-file RON chains (situation, combat / stat /
/// procgen tuning) onto the generic hot-RON registration, GTW-570 moved the seven
/// folder content families (ranged/melee weapons, armor, fields, gangs,
/// terrain + theme defs) onto the generic content-family registration, and GTW-619
/// moved the attachments folder the same way — their kick-off / resolve /
/// redrive now live in `gdtf_assets`, registered by one ext call each
/// in the Load plugin, so their Load-scoped handles left this set. The THEME
/// stays (its kick-off/resolve remain bespoke: the resolve pairs with the fonts
/// folder's recursive load state — the GTW-564 C7 record) but its handle is the
/// generic [`HotRonHandle`]`<GdtfThemeSpec>`, inserted as the PERSISTENT
/// resource by the theme resolve so the UI's generic redrive filters against
/// it. The injuries / prefabs folders stay bespoke — the declared GTW-570
/// exclusions (injuries is one folder → two resources; prefabs is a UUID
/// multimap).
#[derive(Resource, Clone, Debug)]
pub(in crate::states::load) struct LoadHandles {
    /// The theme RON asset being loaded — the generic hot-RON handle the theme
    /// resolve later re-inserts as the PERSISTENT redrive filter.
    pub theme:    HotRonHandle<GdtfThemeSpec>,
    /// The fonts folder being preloaded (all fonts up front).
    pub fonts:    FontFolderHandle,
    /// The injuries folder being preloaded (all `*.injury.ron` + `*.weighting.ron`
    /// up front, GTW-437).
    pub injuries: InjuriesFolderHandle,
    /// The maps (prefab) folder being preloaded (all `*.prefab.ron` fragments up front,
    /// recursively under `<theme>/<size>/`, GTW-489 — the UUID-keyed
    /// [`PrefabSpec`](gdtf_battle_sim::level::PrefabSpec); the ONLY prefab loader after
    /// GTW-494 retired the legacy flat-dir load.).
    pub prefabs:  PrefabsFolderHandle,
}

crate::support_item! {
    /// The resolved authored battlefield — the persistent [`Situation`] resource the
    /// later Generation slice (E10.5) reads on `BattleScapeState::Generation`.
    ///
    /// A named newtype over [`Situation`] (no-bare-types) that [`Deref`]s to it, so a
    /// consumer reads the situation's fields straight through. The poll/resolve system
    /// inserts it once the loaded `RonAsset<Situation>` resolves, and — like
    /// [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) and
    /// the theme's persistent [`HotRonHandle`] — it is the deliberate
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

/// The PERSISTENT handle to the loaded **injuries folder** (`injuries/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that —
/// unlike the Load-scoped [`InjuriesFolderHandle`] inside [`LoadHandles`], which is
/// dropped `OnExit(Load)` — **persists** past `Load` (GTW-437). It is inserted alongside the resolved
/// [`InjuryRegistry`](gdtf_battle_sim::injuries::InjuryRegistry) and
/// [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables) and kept alive so the
/// GTW-437 live hot-reload handler
/// (`redrive_injuries_on_asset_event`)
/// can re-enumerate the folder's member handles to rebuild BOTH resources on a hot edit
/// to ANY `assets/content/injuries/**/*.injury.ron` OR `*.weighting.ron`. Holding the folder
/// handle keeps every member injury asset loaded for the file-watcher. Like the
/// resources, it is **not** removed in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveInjuriesFolderHandle(Handle<LoadedFolder>);

impl ActiveInjuriesFolderHandle {
    /// Wrap the loaded injuries-folder handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the loaded **maps (prefab) folder** (`maps/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that — unlike the
/// Load-scoped [`PrefabsFolderHandle`] inside [`LoadHandles`], which is dropped
/// `OnExit(Load)` — **persists** past `Load` (GTW-489), the prefab mirror of
/// [`ActiveInjuriesFolderHandle`]. It is inserted alongside the resolved
/// [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry) and kept alive so the GTW-489
/// live hot-reload handler (`redrive_prefabs_on_asset_event`) can re-enumerate the
/// folder's member handles to rebuild the registry on a hot edit to ANY
/// `assets/content/maps/**/*.prefab.ron`. Holding the folder handle keeps every member
/// prefab asset loaded for the file-watcher. Like the registry, it is **not** removed
/// in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActivePrefabsFolderHandle(Handle<LoadedFolder>);

impl ActivePrefabsFolderHandle {
    /// Wrap the loaded maps (prefab) folder handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The loose-asset path of a load that reached
/// [`LoadState::Failed`](bevy::asset::LoadState::Failed).
///
/// A named newtype over the path `String` so the failure record names *what*
/// failed in the type, not a bare string.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub(in crate::states::load) struct FailedAssetPath(String);

impl FailedAssetPath {
    /// Record the loose-asset path of a load that failed.
    pub(in crate::states::load) fn new(path: impl Into<String>) -> Self {
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
pub(in crate::states::load) struct LoadFailed(FailedAssetPath);

impl LoadFailed {
    /// Record that a required `Load` asset failed to load, naming its path.
    pub(in crate::states::load) const fn new(path: FailedAssetPath) -> Self {
        Self(path)
    }
}
