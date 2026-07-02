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
use gdtf_battle_sim::{
    procgen::ProcgenTuning,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_ui::theme::GdtfThemeSpec;

/// Typed handle to the in-flight theme RON asset (`core_tuning/ui_theme.tuning.ron`).
///
/// A named newtype over the bevy [`Handle`] so the no-bare-types rule holds even
/// for asset plumbing: a bare `Handle<RonAsset<GdtfThemeSpec>>` carries no domain
/// meaning, this name says "the theme being loaded".
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct ThemeHandle(Handle<RonAsset<GdtfThemeSpec>>);

impl ThemeHandle {
    /// Wrap an in-flight theme RON asset handle.
    pub(in crate::states::load) const fn new(handle: Handle<RonAsset<GdtfThemeSpec>>) -> Self {
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
pub(in crate::states::load) struct FontFolderHandle(Handle<LoadedFolder>);

impl FontFolderHandle {
    /// Wrap an in-flight fonts-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **weapons folder** load (`weapons/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`FontFolderHandle`] (GTW-257). The `Load` scene preloads the whole
/// `assets/content/weapons/ranged/` folder up front via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder); the
/// poll/resolve system gates on its recursive load state, then builds the
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) from the loaded
/// `RonAsset<WeaponSpec>` files (keyed by filename stem). Holding this handle keeps
/// a strong reference to every weapon asset while the registry is built; the
/// registry then holds the specs BY VALUE, so they survive the handle being dropped
/// on `OnExit(Load)`.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct WeaponsFolderHandle(Handle<LoadedFolder>);

impl WeaponsFolderHandle {
    /// Wrap an in-flight weapons-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **melee-weapons folder** load (`weapons/melee/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`WeaponsFolderHandle`] (GTW-505). The `Load` scene preloads the whole
/// `assets/content/weapons/melee/` folder up front via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder); the
/// poll/resolve system gates on its recursive load state, then builds the
/// [`MeleeWeaponRegistry`](gdtf_battle_sim::weapon::MeleeWeaponRegistry) from the loaded
/// `RonAsset<MeleeWeaponSpec>` files (keyed by filename stem). Holding this handle keeps a
/// strong reference to every melee weapon asset while the registry is built; the registry
/// then holds the specs BY VALUE, so they survive the handle being dropped on
/// `OnExit(Load)`.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct MeleeWeaponsFolderHandle(Handle<LoadedFolder>);

impl MeleeWeaponsFolderHandle {
    /// Wrap an in-flight melee-weapons-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **attachments folder** load (`content/attachments/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule), mirroring
/// [`MeleeWeaponsFolderHandle`] (GTW-549, PHASE 1). The `Load` scene preloads the whole
/// `assets/content/attachments/` folder up front via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder); the poll/resolve
/// system gates on its recursive load state, then builds the
/// [`AttachmentRegistry`](gdtf_battle_sim::weapon::AttachmentRegistry) from the loaded
/// `RonAsset<AttachmentSpec>` files (keyed by filename stem). Holding this handle keeps a
/// strong reference to every attachment asset while the registry is built; the registry then
/// holds the specs BY VALUE, so they survive the handle being dropped on `OnExit(Load)`.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct AttachmentsFolderHandle(Handle<LoadedFolder>);

impl AttachmentsFolderHandle {
    /// Wrap an in-flight attachments-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **armor folder** load (`armor/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`WeaponsFolderHandle`] (GTW-269). The `Load` scene preloads the whole
/// `assets/content/armor/` folder up front via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder); the
/// poll/resolve system gates on its recursive load state, then builds the
/// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry) from the loaded
/// `RonAsset<ArmorSpec>` files (keyed by filename stem). Holding this handle keeps a
/// strong reference to every armor asset while the registry is built; the registry
/// then holds the specs BY VALUE, so they survive the handle being dropped on
/// `OnExit(Load)`.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct ArmorsFolderHandle(Handle<LoadedFolder>);

impl ArmorsFolderHandle {
    /// Wrap an in-flight armor-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **area-damage-fields folder** load (`fields/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule), mirroring
/// [`ArmorsFolderHandle`] (GTW-545). The `Load` scene preloads the whole
/// `assets/content/fields/` folder up front via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder); the poll/resolve
/// system gates on its recursive load state, then builds the
/// [`FieldDefRegistry`](gdtf_battle_sim::FieldDefRegistry) from the loaded
/// `RonAsset<FieldDef>` files (keyed by filename stem). Holding this handle keeps a strong
/// reference to every field asset while the registry is built; the registry then holds the
/// defs BY VALUE, so they survive the handle being dropped on `OnExit(Load)`.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct FieldsFolderHandle(Handle<LoadedFolder>);

impl FieldsFolderHandle {
    /// Wrap an in-flight fields-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **injuries folder** load (`injuries/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`WeaponsFolderHandle`] (GTW-437). The `Load` scene preloads the whole
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

/// Typed handle to the in-flight **gangs folder** load (`gangs/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`WeaponsFolderHandle`] (GTW-415). The `Load` scene preloads the whole
/// `assets/content/gangs/` folder up front via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder); the
/// poll/resolve system gates on its recursive load state, then builds the
/// [`GangRegistry`](gdtf_battle_sim::ganger::GangRegistry) from the loaded
/// `RonAsset<GangRoster>` files (keyed by each file's stem, minus the `.gang` infix).
/// Holding this handle keeps a strong reference to every gang asset while the registry
/// is built; the registry then holds the rosters BY VALUE, so they survive the handle
/// being dropped on `OnExit(Load)`.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct GangsFolderHandle(Handle<LoadedFolder>);

impl GangsFolderHandle {
    /// Wrap an in-flight gangs-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **per-theme terrain-model folder** load (`terrain/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`ArmorsFolderHandle`] (GTW-487). The UUID-keyed terrain + theme models
/// ([`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef), GTW-484, and
/// [`UuidThemeDef`](gdtf_battle_sim::level::UuidThemeDef), GTW-485) live under ONE per-theme
/// directory layout — `terrain/<theme>/<tile>.terrain_def.ron` (defs) +
/// `terrain/<theme>/<theme>.terrain_theme.ron` (themes) — so a single recursive
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder) of `terrain/` fans out
/// every member to the matching dedicated-extension loader (`terrain_def.ron` →
/// `RonAsset<TerrainDef>`, `terrain_theme.ron` → `RonAsset<UuidThemeDef>`). The poll/resolve
/// systems gate on this folder's recursive load state, then build BOTH the
/// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) (keyed by each
/// def's OWN [`TerrainUuid`]) and the
/// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) (keyed by each def's OWN
/// [`ThemeUuid`]) from its members. Holding this handle keeps a strong reference to every
/// member asset while the registries are built; the registries then hold the defs BY VALUE,
/// so they survive the handle being dropped on `OnExit(Load)`.
///
/// **GTW-494** — this is the ONLY terrain / theme loader in the Load flow: the legacy
/// flat-dir `content/terrain/` + `content/themes/` loaders were retired (the sim + procgen
/// + presenter consume the UUID-keyed registries as of GTW-491/492/493).
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct TerrainModelFolderHandle(Handle<LoadedFolder>);

impl TerrainModelFolderHandle {
    /// Wrap an in-flight new per-theme terrain-model-folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight **v2 maps (prefab) folder** load (`maps/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types rule),
/// mirroring [`GangsFolderHandle`] (GTW-489 — child T05c of the GTW-476 data-model
/// refactor). The UUID-keyed [`PrefabSpecV2`](gdtf_battle_sim::level::PrefabSpecV2)
/// fragments (GTW-486) live under the nested `assets/content/maps/<theme>/<size>/` tree. Each v2
/// fragment carries the dedicated `prefab_v2.ron` compound extension so the recursive
/// `load_folder` dispatches it to the
/// [`RonAsset<PrefabSpecV2>`](gdtf_assets::RonAsset) loader. The poll/resolve system gates
/// on this folder's recursive load state, then builds the
/// [`PrefabRegistry2`](gdtf_battle_sim::level::PrefabRegistry2) from the loaded
/// `RonAsset<PrefabSpecV2>` members (bucketed by each spec's `(theme, size, role)`, with NO
/// edge-opening validation — the v2 schema has none). Holding this handle keeps a strong
/// reference to every v2 prefab asset while the registry is built; the registry then holds
/// the prefabs BY VALUE, so they survive the handle being dropped on `OnExit(Load)`.
///
/// **GTW-494** — this is the ONLY prefab loader in the Load flow: the legacy flat-dir
/// `content/maps/*.prefab.ron` loader was retired (the procgen pipeline consumes
/// [`PrefabRegistry2`](gdtf_battle_sim::level::PrefabRegistry2) as of GTW-492).
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct PrefabsV2FolderHandle(Handle<LoadedFolder>);

impl PrefabsV2FolderHandle {
    /// Wrap an in-flight maps (v2 prefab) folder load handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight situation RON asset (`content/situations/skirmish.ron`).
///
/// A named newtype over the bevy [`Handle`] so the no-bare-types rule holds even
/// for asset plumbing: a bare `Handle<RonAsset<Situation>>` carries no domain
/// meaning, this name says "the authored battlefield being loaded" (GTW-205 /
/// E10.3). The poll/resolve system reads it to check the load's progress, then
/// resolves it into the persistent [`LoadedSituation`].
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct SituationHandle(Handle<RonAsset<Situation>>);

impl SituationHandle {
    /// Wrap an in-flight situation RON asset handle.
    pub(in crate::states::load) const fn new(handle: Handle<RonAsset<Situation>>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight combat-tuning RON asset (`core_tuning/combat.tuning.ron`).
///
/// A named newtype over the bevy [`Handle`] so the no-bare-types rule holds even
/// for asset plumbing: a bare `Handle<RonAsset<CombatTuning>>` carries no domain
/// meaning, this name says "the shipped combat tuning being loaded" (GTW-206 /
/// E10.4). The poll/resolve system reads it to check the load's progress, then
/// inserts the deserialized [`CombatTuning`] as the persistent runtime resource.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct TuningHandle(Handle<RonAsset<CombatTuning>>);

impl TuningHandle {
    /// Wrap an in-flight combat-tuning RON asset handle.
    pub(in crate::states::load) const fn new(handle: Handle<RonAsset<CombatTuning>>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight ganger stat-tuning RON asset (`core_tuning/stat.tuning.ron`).
///
/// A named newtype over the bevy [`Handle`] so the no-bare-types rule holds even for
/// asset plumbing: a bare `Handle<RonAsset<GangerStatTuning>>` carries no domain meaning,
/// this name says "the shipped ganger stat-derivation tuning being loaded" (GTW-384, the
/// [`TuningHandle`] mirror). The poll/resolve system reads it to check the load's
/// progress, then inserts the deserialized [`GangerStatTuning`] as the persistent runtime
/// resource the sim derives each ganger's computed stats from.
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct StatTuningHandle(Handle<RonAsset<GangerStatTuning>>);

impl StatTuningHandle {
    /// Wrap an in-flight ganger stat-tuning RON asset handle.
    pub(in crate::states::load) const fn new(handle: Handle<RonAsset<GangerStatTuning>>) -> Self {
        Self(handle)
    }
}

/// Typed handle to the in-flight procgen-tuning RON asset (`core_tuning/procgen.tuning.ron`).
///
/// A named newtype over the bevy [`Handle`] so the no-bare-types rule holds even for asset
/// plumbing: a bare `Handle<RonAsset<ProcgenTuning>>` carries no domain meaning, this name says
/// "the shipped procgen fill tuning being loaded" (GTW-533, the [`TuningHandle`] mirror). The
/// poll/resolve system reads it to check the load's progress, then inserts the deserialized
/// [`ProcgenTuning`] as the persistent runtime resource the procgen fill pass reads — closing
/// the previously-documented-but-unwired hot-reload gap (the consumer had used
/// `ProcgenTuning::default()`).
#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct ProcgenTuningHandle(Handle<RonAsset<ProcgenTuning>>);

impl ProcgenTuningHandle {
    /// Wrap an in-flight procgen-tuning RON asset handle.
    pub(in crate::states::load) const fn new(handle: Handle<RonAsset<ProcgenTuning>>) -> Self {
        Self(handle)
    }
}

/// The Load-scoped handles to the assets the [`AppState::Load`](crate::states::AppState::Load)
/// kick-off started loading.
///
/// Holds the typed [`ThemeHandle`], [`FontFolderHandle`], [`SituationHandle`],
/// [`TuningHandle`], [`StatTuningHandle`], [`ProcgenTuningHandle`], [`WeaponsFolderHandle`],
/// [`ArmorsFolderHandle`], [`InjuriesFolderHandle`], [`GangsFolderHandle`],
/// [`PrefabsV2FolderHandle`], and [`TerrainModelFolderHandle`] the poll/resolve system
/// reads each frame to check load progress. Inserted `OnEnter(Load)` and removed
/// `OnExit(Load)` (it has no meaning outside `Load`).
#[derive(Resource, Clone, Debug)]
pub(in crate::states::load) struct LoadHandles {
    /// The theme RON asset being loaded.
    pub theme:         ThemeHandle,
    /// The fonts folder being preloaded (all fonts up front).
    pub fonts:         FontFolderHandle,
    /// The authored situation RON asset being loaded (GTW-205 / E10.3).
    pub situation:     SituationHandle,
    /// The shipped combat-tuning RON asset being loaded (GTW-206 / E10.4).
    pub tuning:        TuningHandle,
    /// The shipped ganger stat-tuning RON asset being loaded (GTW-384).
    pub stat_tuning:   StatTuningHandle,
    /// The shipped procgen fill-tuning RON asset being loaded (GTW-533).
    pub procgen:       ProcgenTuningHandle,
    /// The RANGED-weapons folder being preloaded (all `ranged/*.weapon.ron`s up front,
    /// GTW-257; GTW-505 moved them under `ranged/`).
    pub weapons:       WeaponsFolderHandle,
    /// The MELEE-weapons folder being preloaded (all `melee/*.melee_weapon.ron`s up front,
    /// GTW-505).
    pub melee_weapons: MeleeWeaponsFolderHandle,
    /// The attachments folder being preloaded (all `content/attachments/*.attachment.ron`s up
    /// front, GTW-549 PHASE 1).
    pub attachments:   AttachmentsFolderHandle,
    /// The armor folder being preloaded (all armor `.ron`s up front, GTW-269).
    pub armor:         ArmorsFolderHandle,
    /// The area-damage-fields folder being preloaded (all `*.field.ron` catalog entries up
    /// front, GTW-545).
    pub fields:        FieldsFolderHandle,
    /// The injuries folder being preloaded (all `*.injury.ron` + `*.weighting.ron`
    /// up front, GTW-437).
    pub injuries:      InjuriesFolderHandle,
    /// The gangs folder being preloaded (all `*.gang.ron` rosters up front, GTW-415).
    pub gangs:         GangsFolderHandle,
    /// The maps (v2 prefab) folder being preloaded (all `*.prefab_v2.ron` fragments up
    /// front, recursively under `<theme>/<size>/`, GTW-489 — the UUID-keyed
    /// [`PrefabSpecV2`](gdtf_battle_sim::level::PrefabSpecV2); the ONLY prefab loader after
    /// GTW-494 retired the legacy flat-dir `*.prefab.ron` load).
    pub prefabs_v2:    PrefabsV2FolderHandle,
    /// The per-theme terrain-model folder being preloaded (all `*.terrain_def.ron`
    /// and `*.terrain_theme.ron` files up front, recursively under `terrain/<theme>/`,
    /// GTW-487 — the GTW-484/485 UUID-keyed models; the ONLY terrain / theme loader after
    /// GTW-494 retired the legacy flat-dir `terrain` / `themes` loads).
    pub terrain_model: TerrainModelFolderHandle,
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

/// The PERSISTENT handle to the resolved situation RON asset (`content/situations/skirmish.ron`).
///
/// A named newtype over the bevy [`Handle`] (no-bare-types) that — unlike the
/// Load-scoped [`SituationHandle`] inside [`LoadHandles`], which is dropped
/// `OnExit(Load)` — **persists** past `Load` (GTW-533), mirroring
/// [`ActiveTuningHandle`]. It is inserted alongside the resolved [`LoadedSituation`]
/// resource and kept alive so the GTW-533 live hot-reload handler
/// ([`redrive_situation_on_asset_event`](super::systems::resolve::situation::redrive_situation_on_asset_event))
/// can (1) filter incoming [`AssetEvent`](bevy::asset::AssetEvent) ids against the
/// active situation handle and (2) re-read the refreshed asset out of the `Assets`
/// collection on a hot edit. Holding the handle also keeps a strong reference so the
/// asset stays loaded for the file-watcher. Like [`LoadedSituation`], it is **not**
/// removed in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveSituationHandle(Handle<RonAsset<Situation>>);

impl ActiveSituationHandle {
    /// Wrap the resolved situation RON handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<RonAsset<Situation>>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the resolved combat-tuning RON asset (`core_tuning/combat.tuning.ron`).
///
/// A named newtype over the bevy [`Handle`] (no-bare-types) that — unlike the
/// Load-scoped [`TuningHandle`] inside [`LoadHandles`], which is dropped
/// `OnExit(Load)` — **persists** past `Load`, mirroring
/// [`ActiveThemeHandle`](gdtf_ui::theme::ActiveThemeHandle) (GTW-374). It is inserted
/// alongside the resolved [`CombatTuning`] resource and kept alive so the GTW-374 live
/// hot-reload handler
/// ([`redrive_combat_tuning_on_asset_event`](super::systems::resolve::tuning::redrive_combat_tuning_on_asset_event))
/// can (1) filter incoming [`AssetEvent`](bevy::asset::AssetEvent) ids against the
/// active tuning handle and (2) re-read the refreshed asset out of the `Assets`
/// collection on a hot edit. Holding the handle also keeps a strong reference so the
/// asset stays loaded for the file-watcher. Like [`CombatTuning`], it is **not**
/// removed in `cleanup`. The handler is `redrive_combat_tuning_on_asset_event`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveTuningHandle(Handle<RonAsset<CombatTuning>>);

impl ActiveTuningHandle {
    /// Wrap the resolved combat-tuning RON handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<RonAsset<CombatTuning>>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the resolved ganger stat-tuning RON asset
/// (`core_tuning/stat.tuning.ron`).
///
/// A named newtype over the bevy [`Handle`] (no-bare-types) that — unlike the Load-scoped
/// [`StatTuningHandle`] inside [`LoadHandles`], which is dropped `OnExit(Load)` —
/// **persists** past `Load`, the GTW-384 mirror of [`ActiveTuningHandle`]. It is inserted
/// alongside the resolved [`GangerStatTuning`] resource and kept alive so the live
/// hot-reload handler
/// ([`redrive_stat_tuning_on_asset_event`](super::systems::resolve::stat_tuning::redrive_stat_tuning_on_asset_event))
/// can (1) filter incoming [`AssetEvent`](bevy::asset::AssetEvent) ids against the active
/// stat-tuning handle and (2) re-read the refreshed asset on a hot edit. Holding the
/// handle also keeps a strong reference so the asset stays loaded for the file-watcher.
/// Like [`GangerStatTuning`], it is **not** removed in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveStatTuningHandle(Handle<RonAsset<GangerStatTuning>>);

impl ActiveStatTuningHandle {
    /// Wrap the resolved ganger stat-tuning RON handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<RonAsset<GangerStatTuning>>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the resolved procgen-tuning RON asset
/// (`core_tuning/procgen.tuning.ron`).
///
/// A named newtype over the bevy [`Handle`] (no-bare-types) that — unlike the Load-scoped
/// [`ProcgenTuningHandle`] inside [`LoadHandles`], which is dropped `OnExit(Load)` —
/// **persists** past `Load` (GTW-533), the procgen mirror of [`ActiveTuningHandle`]. It is
/// inserted alongside the resolved [`ProcgenTuning`] resource and kept alive so the live
/// hot-reload handler
/// ([`redrive_procgen_tuning_on_asset_event`](super::systems::resolve::procgen_tuning::redrive_procgen_tuning_on_asset_event))
/// can (1) filter incoming [`AssetEvent`](bevy::asset::AssetEvent) ids against the active
/// procgen-tuning handle and (2) re-read the refreshed asset on a hot edit. Holding the handle
/// also keeps a strong reference so the asset stays loaded for the file-watcher. Like
/// [`ProcgenTuning`], it is **not** removed in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveProcgenTuningHandle(Handle<RonAsset<ProcgenTuning>>);

impl ActiveProcgenTuningHandle {
    /// Wrap the resolved procgen-tuning RON handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<RonAsset<ProcgenTuning>>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the loaded **weapons folder** (`weapons/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that —
/// unlike the Load-scoped [`WeaponsFolderHandle`] inside [`LoadHandles`], which is
/// dropped `OnExit(Load)` — **persists** past `Load` (GTW-374). It is inserted
/// alongside the resolved [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry)
/// and kept alive so the GTW-374 live hot-reload handler
/// (`redrive_weapons_on_asset_event`)
/// can re-enumerate the folder's member handles to rebuild the registry on a hot edit
/// to ANY `assets/content/weapons/ranged/*.weapon.ron`. Holding the folder handle keeps every member
/// weapon asset loaded for the file-watcher. Like the registry, it is **not** removed
/// in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveWeaponsFolderHandle(Handle<LoadedFolder>);

impl ActiveWeaponsFolderHandle {
    /// Wrap the loaded weapons-folder handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the loaded **melee-weapons folder** (`weapons/melee/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that — unlike
/// the Load-scoped [`MeleeWeaponsFolderHandle`] inside [`LoadHandles`], which is dropped
/// `OnExit(Load)` — **persists** past `Load` (GTW-505), the melee mirror of
/// [`ActiveWeaponsFolderHandle`]. It is inserted alongside the resolved
/// [`MeleeWeaponRegistry`](gdtf_battle_sim::weapon::MeleeWeaponRegistry) and kept alive so
/// the live hot-reload handler (`redrive_melee_weapons_on_asset_event`) can re-enumerate
/// the folder's member handles to rebuild the registry on a hot edit to ANY
/// `assets/content/weapons/melee/*.melee_weapon.ron`. Holding the folder handle keeps every
/// member melee weapon asset loaded for the file-watcher. Like the registry, it is **not**
/// removed in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveMeleeWeaponsFolderHandle(Handle<LoadedFolder>);

impl ActiveMeleeWeaponsFolderHandle {
    /// Wrap the loaded melee-weapons-folder handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the loaded **attachments folder** (`content/attachments/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that — unlike the
/// Load-scoped [`AttachmentsFolderHandle`] inside [`LoadHandles`], which is dropped
/// `OnExit(Load)` — **persists** past `Load` (GTW-549, PHASE 1), the attachment mirror of
/// [`ActiveMeleeWeaponsFolderHandle`]. It is inserted alongside the resolved
/// [`AttachmentRegistry`](gdtf_battle_sim::weapon::AttachmentRegistry) and kept alive so the
/// live hot-reload handler (`redrive_attachments_on_asset_event`) can re-enumerate the
/// folder's member handles to rebuild the registry on a hot edit to ANY
/// `assets/content/attachments/*.attachment.ron`. Holding the folder handle keeps every
/// member attachment asset loaded for the file-watcher. Like the registry, it is **not**
/// removed in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveAttachmentsFolderHandle(Handle<LoadedFolder>);

impl ActiveAttachmentsFolderHandle {
    /// Wrap the loaded attachments-folder handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the loaded **armor folder** (`armor/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that —
/// unlike the Load-scoped [`ArmorsFolderHandle`] inside [`LoadHandles`], which is
/// dropped `OnExit(Load)` — **persists** past `Load` (GTW-374), the armor mirror of
/// [`ActiveWeaponsFolderHandle`]. It is inserted alongside the resolved
/// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry) and kept alive so the
/// GTW-374 live hot-reload handler
/// (`redrive_armor_on_asset_event`)
/// can re-enumerate the folder's member handles to rebuild the registry on a hot edit
/// to ANY `assets/content/armor/*.armor.ron`. Holding the folder handle keeps every member
/// armor asset loaded for the file-watcher. Like the registry, it is **not** removed
/// in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveArmorFolderHandle(Handle<LoadedFolder>);

impl ActiveArmorFolderHandle {
    /// Wrap the loaded armor-folder handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the loaded **area-damage-fields folder** (`fields/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that — unlike the
/// Load-scoped [`FieldsFolderHandle`] inside [`LoadHandles`], which is dropped
/// `OnExit(Load)` — **persists** past `Load` (GTW-545), the field mirror of
/// [`ActiveArmorFolderHandle`]. It is inserted alongside the resolved
/// [`FieldDefRegistry`](gdtf_battle_sim::FieldDefRegistry) and kept alive so the live
/// hot-reload handler (`redrive_fields_on_asset_event`) can re-enumerate the folder's member
/// handles to rebuild the catalog on a hot edit to ANY `assets/content/fields/*.field.ron`.
/// Holding the folder handle keeps every member field asset loaded for the file-watcher. Like
/// the registry, it is **not** removed in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveFieldsFolderHandle(Handle<LoadedFolder>);

impl ActiveFieldsFolderHandle {
    /// Wrap the loaded fields-folder handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the loaded **injuries folder** (`injuries/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that —
/// unlike the Load-scoped [`InjuriesFolderHandle`] inside [`LoadHandles`], which is
/// dropped `OnExit(Load)` — **persists** past `Load` (GTW-437), the injuries mirror of
/// [`ActiveWeaponsFolderHandle`]. It is inserted alongside the resolved
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

/// The PERSISTENT handle to the loaded **gangs folder** (`gangs/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that —
/// unlike the Load-scoped [`GangsFolderHandle`] inside [`LoadHandles`], which is
/// dropped `OnExit(Load)` — **persists** past `Load` (GTW-415), the gang mirror of
/// [`ActiveWeaponsFolderHandle`]. It is inserted alongside the resolved
/// [`GangRegistry`](gdtf_battle_sim::ganger::GangRegistry) and kept alive so the GTW-415
/// live hot-reload handler
/// (`redrive_gangs_on_asset_event`)
/// can re-enumerate the folder's member handles to rebuild the registry on a hot edit
/// to ANY `assets/content/gangs/*.gang.ron`. Holding the folder handle keeps every member
/// gang asset loaded for the file-watcher. Like the registry, it is **not** removed
/// in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveGangsFolderHandle(Handle<LoadedFolder>);

impl ActiveGangsFolderHandle {
    /// Wrap the loaded gangs-folder handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the loaded **v2 maps (prefab) folder** (`maps/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that — unlike the
/// Load-scoped [`PrefabsV2FolderHandle`] inside [`LoadHandles`], which is dropped
/// `OnExit(Load)` — **persists** past `Load` (GTW-489), the v2-prefab mirror of
/// [`ActiveGangsFolderHandle`]. It is inserted alongside the resolved
/// [`PrefabRegistry2`](gdtf_battle_sim::level::PrefabRegistry2) and kept alive so the GTW-489
/// live hot-reload handler (`redrive_prefabs_v2_on_asset_event`) can re-enumerate the
/// folder's member handles to rebuild the registry on a hot edit to ANY
/// `assets/content/maps/**/*.prefab_v2.ron`. Holding the folder handle keeps every member v2 prefab
/// asset loaded for the file-watcher. Like the registry, it is **not** removed in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActivePrefabsV2FolderHandle(Handle<LoadedFolder>);

impl ActivePrefabsV2FolderHandle {
    /// Wrap the loaded maps (v2 prefab) folder handle as the persistent hot-reload handle.
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

/// The PERSISTENT handle to the loaded **new per-theme terrain-model folder** (`terrain/`).
///
/// A named newtype over the bevy [`Handle<LoadedFolder>`] (no-bare-types) that — unlike the
/// Load-scoped [`TerrainModelFolderHandle`] inside [`LoadHandles`], which is dropped
/// `OnExit(Load)` — **persists** past `Load` (GTW-487), the new-model mirror of
/// [`ActiveArmorFolderHandle`]. It is inserted alongside the resolved
/// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
/// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) and kept alive so the
/// GTW-487 live hot-reload handlers (`redrive_terrain_defs_on_asset_event` /
/// `redrive_theme_defs_on_asset_event`) can re-enumerate the folder's member handles to
/// rebuild each registry on a hot edit to ANY `assets/terrain/**/*.terrain_def.ron` OR
/// `*.terrain_theme.ron`. Holding the folder handle keeps every member asset loaded for the
/// file-watcher. Like the registries, it is **not** removed in `cleanup`.
#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveTerrainModelFolderHandle(Handle<LoadedFolder>);

impl ActiveTerrainModelFolderHandle {
    /// Wrap the loaded new per-theme terrain-model-folder handle as the persistent
    /// hot-reload handle.
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
