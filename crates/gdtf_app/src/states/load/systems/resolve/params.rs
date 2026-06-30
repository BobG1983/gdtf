//! The `Update`-poll [`SystemParam`] bundles: the loaded RON asset collections
//! [`poll_and_resolve`](super::poll_and_resolve) reads, and the persistent resources it
//! resolves — each grouped into one bundle so the system's parameter list stays under
//! clippy's argument-count gate.

use bevy::{
    asset::LoadedFolder,
    ecs::system::SystemParam,
    prelude::{Assets, Res},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    armor::{ArmorRegistry, ArmorSpec},
    ganger::{GangRegistry, GangRoster},
    injuries::{InjuryDef, InjuryRegistry, InjuryWeighting},
    level::{PrefabRegistry2, PrefabSpecV2, UuidThemeDef, UuidThemeRegistry},
    situation::Situation,
    terrain::def::{TerrainDef, TerrainDefRegistry},
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{MeleeWeaponRegistry, MeleeWeaponSpec, WeaponRegistry, WeaponSpec},
};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

use crate::states::load::resources::LoadedSituation;

/// The loaded RON asset collections [`poll_and_resolve`](super::poll_and_resolve)
/// reads, bundled into one [`SystemParam`] so the system's parameter list stays under
/// clippy's argument-count gate (the [`BattleGridsParam`](gdtf_battle_sim) grouping
/// precedent — a transparent bundle of existing world-state resources, not a
/// wrapped domain scalar).
///
/// Each is `Option<Res<…>>` because a `MinimalPlugins` headless app has no
/// `AssetServer` (and so no `Assets<…>` collections); the system early-returns
/// when any is absent, so it never panics on a missing collection (bevy-traps
/// rule 1).
#[derive(SystemParam)]
pub(in crate::states::load) struct LoadAssetCollections<'w> {
    /// The loaded theme-spec RON collection (`core_tuning/ui_theme.tuning.ron`).
    pub(super) theme:           Option<Res<'w, Assets<RonAsset<GdtfThemeSpec>>>>,
    /// The loaded authored-situation RON collection (`situations/skirmish.ron`).
    pub(super) situation:       Option<Res<'w, Assets<RonAsset<Situation>>>>,
    /// The loaded combat-tuning RON collection (`core_tuning/combat.tuning.ron`, GTW-206).
    pub(super) tuning:          Option<Res<'w, Assets<RonAsset<CombatTuning>>>>,
    /// The loaded ganger stat-tuning RON collection (`core_tuning/stat.tuning.ron`, GTW-384).
    pub(super) stat_tuning:     Option<Res<'w, Assets<RonAsset<GangerStatTuning>>>>,
    /// The loaded `LoadedFolder` collection — used to read the weapons folder's
    /// member handles when building the [`WeaponRegistry`] (GTW-257).
    pub(super) folders:         Option<Res<'w, Assets<LoadedFolder>>>,
    /// The loaded per-RANGED-weapon RON collection (`weapons/ranged/*.weapon.ron`, GTW-257).
    pub(super) weapon_specs:    Option<Res<'w, Assets<RonAsset<WeaponSpec>>>>,
    /// The loaded per-MELEE-weapon RON collection (`weapons/melee/*.melee_weapon.ron`,
    /// GTW-505).
    pub(super) melee_specs:     Option<Res<'w, Assets<RonAsset<MeleeWeaponSpec>>>>,
    /// The loaded per-armor RON collection (`armor/*.ron`, GTW-269).
    pub(super) armor_specs:     Option<Res<'w, Assets<RonAsset<ArmorSpec>>>>,
    /// The loaded per-injury RON collection (`injuries/**/*.injury.ron`, GTW-437).
    pub(super) injury_defs:     Option<Res<'w, Assets<RonAsset<InjuryDef>>>>,
    /// The loaded per-part injury-weighting RON collection
    /// (`injuries/weighting/*.weighting.ron`, GTW-437).
    pub(super) weightings:      Option<Res<'w, Assets<RonAsset<InjuryWeighting>>>>,
    /// The loaded per-gang roster RON collection (`gangs/*.gang.ron`, GTW-415).
    pub(super) gang_rosters:    Option<Res<'w, Assets<RonAsset<GangRoster>>>>,
    /// The loaded per-prefab v2 RON collection (`maps/**/*.prefab_v2.ron`, GTW-489).
    pub(super) prefab_v2_specs: Option<Res<'w, Assets<RonAsset<PrefabSpecV2>>>>,
    /// The loaded NEW per-theme terrain-def RON collection
    /// (`terrain/<theme>/*.terrain_def.ron`, GTW-487).
    pub(super) terrain_defs:    Option<Res<'w, Assets<RonAsset<TerrainDef>>>>,
    /// The loaded NEW per-theme theme-def RON collection
    /// (`terrain/<theme>/*.terrain_theme.ron`, GTW-487).
    pub(super) theme_defs:      Option<Res<'w, Assets<RonAsset<UuidThemeDef>>>>,
}

/// The four persistent resources [`poll_and_resolve`](super::poll_and_resolve) resolves,
/// each as an `Option<Res<…>>` presence-probe, bundled into one [`SystemParam`] so the
/// system's parameter list stays under clippy's argument-count gate (the
/// [`LoadAssetCollections`] grouping precedent — a transparent bundle of existing
/// world-state resources, not a wrapped domain scalar).
///
/// Each branch resolves on its OWN resource's absence (so none starves another,
/// bevy-traps rule 3); the bundle exposes that per-branch "already present?" probe.
#[derive(SystemParam)]
pub(in crate::states::load) struct ResolvedResources<'w> {
    /// Whether the resolved [`GdtfTheme`] is already inserted.
    pub(super) theme:         Option<Res<'w, GdtfTheme>>,
    /// Whether the resolved [`CombatTuning`] is already inserted (GTW-206).
    pub(super) tuning:        Option<Res<'w, CombatTuning>>,
    /// Whether the resolved [`GangerStatTuning`] is already inserted (GTW-384).
    pub(super) stat_tuning:   Option<Res<'w, GangerStatTuning>>,
    /// Whether the resolved [`WeaponRegistry`] is already inserted (GTW-257).
    pub(super) weapons:       Option<Res<'w, WeaponRegistry>>,
    /// Whether the resolved [`MeleeWeaponRegistry`] is already inserted (GTW-505).
    pub(super) melee_weapons: Option<Res<'w, MeleeWeaponRegistry>>,
    /// Whether the resolved [`LoadedSituation`] is already inserted (GTW-261).
    pub(super) situation:     Option<Res<'w, LoadedSituation>>,
    /// Whether the resolved [`ArmorRegistry`] is already inserted (GTW-269).
    pub(super) armor:         Option<Res<'w, ArmorRegistry>>,
    /// Whether the resolved [`InjuryRegistry`] is already inserted (GTW-437). The
    /// [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables) is built and inserted
    /// in the SAME branch, so the registry's presence is the branch's done-probe.
    pub(super) injuries:      Option<Res<'w, InjuryRegistry>>,
    /// Whether the resolved [`GangRegistry`] is already inserted (GTW-415).
    pub(super) gangs:         Option<Res<'w, GangRegistry>>,
    /// Whether the resolved [`PrefabRegistry2`] is already inserted (GTW-489).
    pub(super) prefabs_v2:    Option<Res<'w, PrefabRegistry2>>,
    /// Whether the resolved [`TerrainDefRegistry`] is already inserted (GTW-487).
    pub(super) terrain_defs:  Option<Res<'w, TerrainDefRegistry>>,
    /// Whether the resolved [`UuidThemeRegistry`] is already inserted (GTW-487).
    pub(super) theme_defs:    Option<Res<'w, UuidThemeRegistry>>,
}
