//! The editor's registration of the GTW-582 validation pass (GTW-630) — the
//! authoring-time twin of the game's `add_content_validation`.

use bevy::{
    ecs::schedule::SystemCondition as _,
    prelude::{App, IntoScheduleConfigs, Res, Update, not, resource_exists},
};
use gdtf_assets::{ContentChecksComplete, ContentValidationAppExt, ContentValidationSet};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    ganger::GangRegistry,
    level::UuidThemeRegistry,
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::validate::{
    check_emplacement_weapon_refs, check_gang_equipment_refs, check_theme_terrain_refs,
};

use super::rearm::rearm_validation_on_content_change;

/// The editor's validation WINDOW condition: every registry the editor's
/// REGISTERED checks read is present. Since the gang equipment edge joined
/// (GTW-651) that is ALL SIX editor-loaded registries — weapons + melee
/// weapons + armor + gangs (the gang edge's reads) plus terrain defs + theme
/// defs (the theme/emplacement edges') — each inserted on its load's success
/// OR its fail-closed empty fallback, so this always eventually opens.
pub(super) const fn validation_graph_ready(
    weapons: Option<Res<WeaponRegistry>>,
    melee_weapons: Option<Res<MeleeWeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    gangs: Option<Res<GangRegistry>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
) -> bool {
    weapons.is_some()
        && melee_weapons.is_some()
        && armor.is_some()
        && gangs.is_some()
        && terrain.is_some()
        && themes.is_some()
}

/// Install the reference-integrity pass on the editor: the seam plumbing
/// ([`ContentValidationAppExt::init_content_validation`]), the `Check`-set
/// window (every read registry resolved, not yet checked), one
/// [`register_reference_check`](ContentValidationAppExt::register_reference_check)
/// hook per editor-loaded edge, and the [`rearm`](super::rearm) system that
/// re-opens the pass on a live content change.
///
/// Unlike the game's window, the editor's is NOT state-gated: the registries
/// persist for the whole session (GTW-533) and the pass re-arms on hot-reload,
/// so the window is simply "readable + unchecked" — validation runs whenever
/// content is (re)loaded, which IS authoring time. Nothing in the editor gates
/// on [`ContentValidationDone`](gdtf_assets::ContentValidationDone) (the
/// `Load → Editing` transition is untouched), so the pass can never strand the
/// editor.
pub(crate) fn register_validation(app: &mut App) {
    app.init_content_validation();
    app.configure_sets(
        Update,
        ContentValidationSet::Check
            .run_if(not(resource_exists::<ContentChecksComplete>).and_then(validation_graph_ready)),
    );
    // The three host-agnostic edge checks over families the editor loads — the
    // SAME systems the game registers (gdtf_content_families::validate). The
    // gang equipment edge joined with the GTW-636 Gang mode's registries
    // (GTW-651).
    app.register_reference_check(check_theme_terrain_refs)
        .register_reference_check(check_emplacement_weapon_refs)
        .register_reference_check(check_gang_equipment_refs);
    app.add_systems(Update, rearm_validation_on_content_change);
}
