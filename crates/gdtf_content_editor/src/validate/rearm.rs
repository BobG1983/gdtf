//! The LIVE half of authoring-time validation (GTW-630): re-arm the pass when
//! a watched registry is rebuilt by the hot-reload redrive.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Commands, DetectChanges, Res},
};
use gdtf_assets::{ContentChecksComplete, ContentIntegrityReport, ContentValidationDone};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    ganger::GangRegistry,
    injuries::InjuryRegistry,
    level::UuidThemeRegistry,
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

/// The WATCH SET: every registry the editor's registered checks read (the
/// [`register`](super::register) window's exact resource set — all seven since
/// the gang equipment edge joined in GTW-651 and the injury-weighting edge in
/// GTW-654), bundled into one
/// `#[derive(SystemParam)]` (the load gate's `GateResources` pattern) so the
/// re-arm system's signature stays legible as families accrue. Every field is
/// `Option` — a registry arrives only once its seam resolve (or fallback)
/// fires (`bevy-traps.md` #1).
#[derive(SystemParam)]
pub(super) struct WatchedRegistries<'w> {
    /// The ranged-weapons registry — read by the emplacement AND gang edges.
    weapons:       Option<Res<'w, WeaponRegistry>>,
    /// The melee-weapons registry — read by the gang edge (GTW-651).
    melee_weapons: Option<Res<'w, MeleeWeaponRegistry>>,
    /// The armor registry — read by the gang edge (GTW-651).
    armor:         Option<Res<'w, ArmorRegistry>>,
    /// The gangs registry — the gang edge's referrer side (GTW-651).
    gangs:         Option<Res<'w, GangRegistry>>,
    /// The UUID-keyed terrain defs — read by the theme + emplacement edges.
    terrain:       Option<Res<'w, TerrainDefRegistry>>,
    /// The UUID-keyed theme defs — the theme edge's referrer side.
    themes:        Option<Res<'w, UuidThemeRegistry>>,
    /// The injury-def registry — read by the weighting edge (GTW-654). The
    /// injuries redrive overwrites the registry AND the tables together on ANY
    /// member edit (def OR weighting), so watching the registry alone re-arms
    /// on both artifact kinds; the built `InjuryTables` is read by no check,
    /// so per the seam invariant it is not watched.
    injuries:      Option<Res<'w, InjuryRegistry>>,
}

impl WatchedRegistries<'_> {
    /// Whether ANY watched registry was rebuilt since the re-arm system last
    /// ran — the hot-reload redrive overwrites a registry through `ResMut`,
    /// which marks it changed.
    fn any_changed(&self) -> bool {
        self.weapons.as_ref().is_some_and(DetectChanges::is_changed)
            || self
                .melee_weapons
                .as_ref()
                .is_some_and(DetectChanges::is_changed)
            || self.armor.as_ref().is_some_and(DetectChanges::is_changed)
            || self.gangs.as_ref().is_some_and(DetectChanges::is_changed)
            || self.terrain.as_ref().is_some_and(DetectChanges::is_changed)
            || self.themes.as_ref().is_some_and(DetectChanges::is_changed)
            || self
                .injuries
                .as_ref()
                .is_some_and(DetectChanges::is_changed)
    }
}

/// `Update` (unconditional): once the pass has published
/// ([`ContentValidationDone`] present), a change to ANY registry the editor's
/// registered checks read ([`WatchedRegistries`]) re-arms it — the report is
/// replaced with a fresh empty one and both monotonic markers are removed, so
/// the seam's `Check` → `Publish` chain runs again over the CURRENT content
/// and re-publishes one consolidated report (the game's report shape,
/// re-emitted at the edit). One watch set, one report: an edit to EITHER side
/// of an edge — the gang file OR the weapons/armor/melee folder it references
/// — re-runs EVERY registered check (GTW-651 A2).
///
/// The reset-don't-accumulate choice: re-checking into the old report would
/// duplicate every still-dangling finding on each edit; replacing it keeps the
/// published report a truthful snapshot of the current registries. (Load-time
/// `MalformedFile` salvage findings are dropped by the reset too — a member
/// that is STILL malformed keeps its stale registry entry or stays absent, and
/// any reference to it dangles, so the mistake stays visible.)
///
/// Deliberately UNGATED (no `run_if`): the system runs every frame, so its
/// change-detection window never stalls — a condition-gated version would not
/// evaluate while [`ContentValidationDone`] is absent, and its first
/// evaluation after the publish would still see the registries' INITIAL
/// insertion ticks as "changed", spuriously re-arming a freshly published
/// pass (`bevy-traps.md` #3, the tick-staleness family). The early return
/// below still consumes the ticks.
pub(super) fn rearm_validation_on_content_change(
    done: Option<Res<ContentValidationDone>>,
    watched: WatchedRegistries,
    mut commands: Commands,
) {
    if done.is_none() {
        return;
    }
    if watched.any_changed() {
        commands.insert_resource(ContentIntegrityReport::default());
        commands.remove_resource::<ContentChecksComplete>();
        commands.remove_resource::<ContentValidationDone>();
    }
}
