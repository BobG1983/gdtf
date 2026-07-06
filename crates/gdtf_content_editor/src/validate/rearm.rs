//! The LIVE half of authoring-time validation (GTW-630): re-arm the pass when
//! a watched registry is rebuilt by the hot-reload redrive.

use bevy::prelude::{Commands, DetectChanges as _, Res};
use gdtf_assets::{ContentChecksComplete, ContentIntegrityReport, ContentValidationDone};
use gdtf_battle_sim::{
    level::UuidThemeRegistry, terrain::def::TerrainDefRegistry, weapon::WeaponRegistry,
};

/// `Update` (unconditional): once the pass has published
/// ([`ContentValidationDone`] present), a change to ANY registry the editor's
/// registered checks read re-arms it — the report is replaced with a fresh
/// empty one and both monotonic markers are removed, so the seam's `Check` →
/// `Publish` chain runs again over the CURRENT content and re-publishes one
/// consolidated report (the game's report shape, re-emitted at the edit).
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
    weapons: Option<Res<WeaponRegistry>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    mut commands: Commands,
) {
    if done.is_none() {
        return;
    }
    let content_changed = weapons.is_some_and(|registry| registry.is_changed())
        || terrain.is_some_and(|registry| registry.is_changed())
        || themes.is_some_and(|registry| registry.is_changed());
    if content_changed {
        commands.insert_resource(ContentIntegrityReport::default());
        commands.remove_resource::<ContentChecksComplete>();
        commands.remove_resource::<ContentValidationDone>();
    }
}
