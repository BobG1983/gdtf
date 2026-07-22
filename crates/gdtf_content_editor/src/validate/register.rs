//! The editor's registration of the GTW-582 validation pass (GTW-630) — the
//! authoring-time twin of the game's `add_content_validation`.

use bevy::{
    ecs::schedule::SystemCondition as _,
    prelude::{App, IntoScheduleConfigs, Res, Update, not, resource_exists},
};
use gdtf_assets::{ContentChecksComplete, ContentValidationAppExt, ContentValidationSet};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::InjuryRegistry,
    level::UuidThemeRegistry,
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::{
    sprites::SpriteDefRegistry,
    validate::{
        check_emplacement_weapon_refs, check_gang_equipment_refs, check_injury_weighting_refs,
        check_terrain_graphic_refs, check_theme_terrain_refs, check_weapon_attachment_refs,
    },
};

use super::rearm::rearm_validation_on_content_change;

/// Presence-probes over every registry the editor's REGISTERED checks read —
/// bundled into one [`SystemParam`](bevy::ecs::system::SystemParam) (the
/// game registrar's `ReferenceGraphResources` pattern) so the
/// [`validation_graph_ready`] window condition stays under clippy's
/// argument-count gate as families accrue.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct ValidationGraphResources<'w> {
    /// The ranged weapons (gang + emplacement edge reads).
    weapons:       Option<Res<'w, WeaponRegistry>>,
    /// The melee weapons (gang edge reads).
    melee_weapons: Option<Res<'w, MeleeWeaponRegistry>>,
    /// The armor (gang edge reads).
    armor:         Option<Res<'w, ArmorRegistry>>,
    /// The gang rosters (the gang edge's referrer side).
    gangs:         Option<Res<'w, GangRegistry>>,
    /// The terrain defs (theme/emplacement edge reads + the `graphic_name`
    /// edge's referrer side).
    terrain:       Option<Res<'w, TerrainDefRegistry>>,
    /// The theme defs (the theme edge's referrer side).
    themes:        Option<Res<'w, UuidThemeRegistry>>,
    /// The injury defs (weighting edge reads).
    injuries:      Option<Res<'w, InjuryRegistry>>,
    /// The sprite defs (`graphic_name` edge reads — GTW-663).
    sprite_defs:   Option<Res<'w, SpriteDefRegistry>>,
    /// The attachment items (the weapon→attachment edge reads — GTW-669).
    attachments:   Option<Res<'w, AttachmentRegistry>>,
}

/// The editor's validation WINDOW condition: every registry the editor's
/// REGISTERED checks read is present. Since the gang equipment edge joined
/// (GTW-651), the injuries edge followed (GTW-654), the terrain
/// `graphic_name` edge joined (GTW-663), and the weapon→attachment edge
/// joined (GTW-669) that is ALL NINE editor-loaded
/// check-read registries — weapons + melee weapons + armor + gangs (the gang
/// edge's reads) plus terrain defs + theme defs (the theme/emplacement
/// edges') plus the injury defs (the weighting edge's) plus the sprite defs
/// (the `graphic_name` edge's) plus the attachments (the weapon edge's) —
/// each inserted on its load's success OR its
/// fail-closed empty fallback, so this always eventually opens.
pub(super) const fn validation_graph_ready(graph: ValidationGraphResources) -> bool {
    graph.weapons.is_some()
        && graph.melee_weapons.is_some()
        && graph.armor.is_some()
        && graph.gangs.is_some()
        && graph.terrain.is_some()
        && graph.themes.is_some()
        && graph.injuries.is_some()
        && graph.sprite_defs.is_some()
        && graph.attachments.is_some()
}

/// Install the reference-integrity pass on the editor: the base plumbing
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
    // The six host-agnostic edge checks over families the editor loads — the
    // SAME systems the game registers (gdtf_content_families::validate). The
    // gang equipment edge joined with the GTW-636 Gang mode's registries
    // (GTW-651); the injury-weighting edge with the GTW-654 Injury mode's;
    // the terrain graphic_name edge with the GTW-663 sprite-defs family; the
    // weapon→attachment edge with the GTW-669 Attachment mode's registry.
    app.register_reference_check(check_theme_terrain_refs)
        .register_reference_check(check_emplacement_weapon_refs)
        .register_reference_check(check_gang_equipment_refs)
        .register_reference_check(check_injury_weighting_refs)
        .register_reference_check(check_terrain_graphic_refs)
        .register_reference_check(check_weapon_attachment_refs);
    app.add_systems(Update, rearm_validation_on_content_change);
}
