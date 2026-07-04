//! The `Update`-poll [`SystemParam`] bundles: the loaded RON asset collections
//! [`poll_and_resolve`](super::poll_and_resolve) reads, and the persistent resources it
//! resolves — each grouped into one bundle so the system's parameter list stays under
//! clippy's argument-count gate.

use bevy::{
    asset::LoadedFolder,
    ecs::system::SystemParam,
    prelude::{Assets, Res, ResMut},
};
use gdtf_assets::{ContentIntegrityReport, RonAsset, RonFolderSalvage};
use gdtf_battle_sim::{
    injuries::{InjuryDef, InjuryRegistry, InjuryWeighting},
    level::{PrefabRegistry, PrefabSpec},
    weapon::{AttachmentRegistry, AttachmentSpec},
};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

/// The loaded RON asset collections [`poll_and_resolve`](super::poll_and_resolve)
/// reads, bundled into one [`SystemParam`] so the system's parameter list stays under
/// clippy's argument-count gate (the [`BattleGridsParam`](gdtf_battle_sim) grouping
/// precedent — a transparent bundle of existing world-state resources, not a
/// wrapped domain scalar).
///
/// GTW-570 shrank this bundle to the BESPOKE branches (attachments / injuries /
/// prefabs — the declared exclusions): the seven generic content families read
/// their collections through their own generic resolve systems now.
///
/// Each is `Option<Res<…>>` because a `MinimalPlugins` headless app has no
/// `AssetServer` (and so no `Assets<…>` collections); the system early-returns
/// when any is absent, so it never panics on a missing collection (bevy-traps
/// rule 1).
#[derive(SystemParam)]
pub(in crate::states::load) struct LoadAssetCollections<'w> {
    /// The loaded theme-spec RON collection (`core_tuning/ui_theme.tuning.ron`).
    pub(super) theme:            Option<Res<'w, Assets<RonAsset<GdtfThemeSpec>>>>,
    /// The loaded `LoadedFolder` collection — used to read each folder's
    /// member handles when building a registry (GTW-257).
    pub(super) folders:          Option<Res<'w, Assets<LoadedFolder>>>,
    /// The loaded per-attachment RON collection (`content/attachments/*.attachment.ron`,
    /// GTW-549 PHASE 1).
    pub(super) attachment_specs: Option<Res<'w, Assets<RonAsset<AttachmentSpec>>>>,
    /// The loaded per-injury RON collection (`injuries/**/*.injury.ron`, GTW-437).
    pub(super) injury_defs:      Option<Res<'w, Assets<RonAsset<InjuryDef>>>>,
    /// The loaded per-part injury-weighting RON collection
    /// (`injuries/weighting/*.weighting.ron`, GTW-437).
    pub(super) weightings:       Option<Res<'w, Assets<RonAsset<InjuryWeighting>>>>,
    /// The loaded per-prefab RON collection (`content/maps/**/*.prefab.ron`, GTW-489).
    pub(super) prefab_specs:     Option<Res<'w, Assets<RonAsset<PrefabSpec>>>>,
}

/// The persistent resources [`poll_and_resolve`](super::poll_and_resolve) resolves,
/// each as an `Option<Res<…>>` presence-probe, bundled into one [`SystemParam`] so the
/// system's parameter list stays under clippy's argument-count gate (the
/// [`LoadAssetCollections`] grouping precedent — a transparent bundle of existing
/// world-state resources, not a wrapped domain scalar).
///
/// Each branch resolves on its OWN resource's absence (so none starves another,
/// bevy-traps rule 3); the bundle exposes that per-branch "already present?" probe.
/// Like [`LoadAssetCollections`], GTW-570 shrank it to the bespoke branches.
#[derive(SystemParam)]
pub(in crate::states::load) struct ResolvedResources<'w> {
    /// Whether the resolved [`GdtfTheme`] is already inserted.
    pub(super) theme:       Option<Res<'w, GdtfTheme>>,
    /// Whether the resolved [`AttachmentRegistry`] is already inserted (GTW-549 PHASE 1).
    pub(super) attachments: Option<Res<'w, AttachmentRegistry>>,
    /// Whether the resolved [`InjuryRegistry`] is already inserted (GTW-437). The
    /// [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables) is built and inserted
    /// in the SAME branch, so the registry's presence is the branch's done-probe.
    pub(super) injuries:    Option<Res<'w, InjuryRegistry>>,
    /// Whether the resolved [`PrefabRegistry`] is already inserted (GTW-489).
    pub(super) prefabs:     Option<Res<'w, PrefabRegistry>>,
}

/// The GTW-582 per-file salvage states of the three BESPOKE folder branches, plus
/// the shared content-integrity report — bundled into one [`SystemParam`] so
/// [`poll_and_resolve`](super::poll_and_resolve)'s parameter list stays under
/// clippy's argument-count gate (the [`LoadAssetCollections`] grouping precedent).
///
/// Each salvage is `Option<Res<…>>`: absent until that branch's folder walk
/// FAILED and the branch began salvaging its members per-file (the C4
/// fail-closed-per-file contract, shared with the generic seam through
/// `gdtf_assets`). The report is `Option<ResMut<…>>` (bevy-traps rule 1) — the
/// Load plugin init's it unconditionally, but a bespoke harness might not.
#[derive(SystemParam)]
pub(in crate::states::load) struct SalvageStates<'w> {
    /// The attachments branch's per-file salvage (GTW-549 folder).
    pub(super) attachments:       Option<Res<'w, RonFolderSalvage<AttachmentSpec>>>,
    /// The injuries branch's per-injury-def salvage (GTW-437 folder, defs half).
    pub(super) injury_defs:       Option<Res<'w, RonFolderSalvage<InjuryDef>>>,
    /// The injuries branch's per-weighting salvage (GTW-437 folder, tables half).
    pub(super) injury_weightings: Option<Res<'w, RonFolderSalvage<InjuryWeighting>>>,
    /// The prefabs branch's per-fragment salvage (GTW-489 folder).
    pub(super) prefabs:           Option<Res<'w, RonFolderSalvage<PrefabSpec>>>,
    /// The shared content-integrity report the salvages record malformed
    /// members into (GTW-582 C2/C4).
    pub(super) report:            Option<ResMut<'w, ContentIntegrityReport>>,
}
