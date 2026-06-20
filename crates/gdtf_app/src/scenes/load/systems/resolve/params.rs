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
    situation::Situation,
    tuning::CombatTuning,
    weapon::{WeaponRegistry, WeaponSpec},
};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

use crate::scenes::load::resources::LoadedSituation;

/// The three loaded RON asset collections [`poll_and_resolve`](super::poll_and_resolve)
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
pub(in crate::scenes::load) struct LoadAssetCollections<'w> {
    /// The loaded theme-spec RON collection (`theme/grimdark.ron`).
    pub(super) theme:        Option<Res<'w, Assets<RonAsset<GdtfThemeSpec>>>>,
    /// The loaded authored-situation RON collection (`situations/skirmish.ron`).
    pub(super) situation:    Option<Res<'w, Assets<RonAsset<Situation>>>>,
    /// The loaded combat-tuning RON collection (`combat/tuning.ron`, GTW-206).
    pub(super) tuning:       Option<Res<'w, Assets<RonAsset<CombatTuning>>>>,
    /// The loaded `LoadedFolder` collection — used to read the weapons folder's
    /// member handles when building the [`WeaponRegistry`] (GTW-257).
    pub(super) folders:      Option<Res<'w, Assets<LoadedFolder>>>,
    /// The loaded per-weapon RON collection (`weapons/*.ron`, GTW-257).
    pub(super) weapon_specs: Option<Res<'w, Assets<RonAsset<WeaponSpec>>>>,
    /// The loaded per-armor RON collection (`armor/*.ron`, GTW-269).
    pub(super) armor_specs:  Option<Res<'w, Assets<RonAsset<ArmorSpec>>>>,
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
pub(in crate::scenes::load) struct ResolvedResources<'w> {
    /// Whether the resolved [`GdtfTheme`] is already inserted.
    pub(super) theme:     Option<Res<'w, GdtfTheme>>,
    /// Whether the resolved [`CombatTuning`] is already inserted (GTW-206).
    pub(super) tuning:    Option<Res<'w, CombatTuning>>,
    /// Whether the resolved [`WeaponRegistry`] is already inserted (GTW-257).
    pub(super) weapons:   Option<Res<'w, WeaponRegistry>>,
    /// Whether the resolved [`LoadedSituation`] is already inserted (GTW-261).
    pub(super) situation: Option<Res<'w, LoadedSituation>>,
    /// Whether the resolved [`ArmorRegistry`] is already inserted (GTW-269).
    pub(super) armor:     Option<Res<'w, ArmorRegistry>>,
}
