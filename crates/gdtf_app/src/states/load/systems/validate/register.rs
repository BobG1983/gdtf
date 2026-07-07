//! The GTW-582 validation-pass **registration surface**: the one call the Load
//! plugin makes to install the whole reference-integrity pass, plus the window
//! condition that opens it once every gate registry has resolved.

use bevy::{
    ecs::{schedule::SystemCondition as _, system::SystemParam},
    prelude::{App, IntoScheduleConfigs, Res, Update, in_state, not, resource_exists},
};
use gdtf_assets::{ContentChecksComplete, ContentValidationAppExt, ContentValidationSet};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    effects::fields::FieldDefRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::InjuryRegistry,
    level::{PrefabRegistry, UuidThemeRegistry},
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

use super::{prefabs, situation};
use crate::states::{AppState, load::resources::LoadedSituation};

/// Presence-probes over every resource the per-edge checks read — bundled into
/// one [`SystemParam`] so the [`reference_graph_ready`] window condition stays
/// under clippy's argument-count gate (the `LoadAssetCollections` precedent).
///
/// This is deliberately the SAME set the `transition_to_intro` gate requires
/// (minus the theme/tunings the checks never read): the checks take plain
/// `Res<…>` because the `Check` set only opens once every probe here is `Some`
/// (a set's run condition is evaluated once and gates every member — the
/// bevy-traps #1 guard lives HERE, once, instead of on each check).
#[derive(SystemParam)]
pub(super) struct ReferenceGraphResources<'w> {
    /// The resolved authored situation (the graph's root).
    situation:     Option<Res<'w, LoadedSituation>>,
    /// The gang rosters (situation → gang/member refs).
    gangs:         Option<Res<'w, GangRegistry>>,
    /// The ranged weapons (member / emplacement refs).
    weapons:       Option<Res<'w, WeaponRegistry>>,
    /// The melee weapons (member refs + the implicit `fists` default).
    melee_weapons: Option<Res<'w, MeleeWeaponRegistry>>,
    /// The armor (member refs).
    armor:         Option<Res<'w, ArmorRegistry>>,
    /// The attachment items (weapon refs).
    attachments:   Option<Res<'w, AttachmentRegistry>>,
    /// The area-damage-field catalog (situation field refs).
    fields:        Option<Res<'w, FieldDefRegistry>>,
    /// The injury defs (weighting-row refs).
    injuries:      Option<Res<'w, InjuryRegistry>>,
    /// The terrain defs (situation / theme / prefab refs).
    terrain:       Option<Res<'w, TerrainDefRegistry>>,
    /// The sprite defs (terrain `graphic_name` refs — GTW-663).
    sprite_defs:   Option<Res<'w, SpriteDefRegistry>>,
    /// The themes (situation / prefab refs).
    themes:        Option<Res<'w, UuidThemeRegistry>>,
    /// The prefab fragments (theme / terrain refs).
    prefabs:       Option<Res<'w, PrefabRegistry>>,
}

/// The validation WINDOW condition: every resource the reference graph spans
/// has resolved (each is inserted on its load's success OR failure fallback,
/// so this always eventually opens — the no-strand guarantee holds).
pub(super) const fn reference_graph_ready(graph: ReferenceGraphResources) -> bool {
    graph.situation.is_some()
        && graph.gangs.is_some()
        && graph.weapons.is_some()
        && graph.melee_weapons.is_some()
        && graph.armor.is_some()
        && graph.attachments.is_some()
        && graph.fields.is_some()
        && graph.injuries.is_some()
        && graph.terrain.is_some()
        && graph.sprite_defs.is_some()
        && graph.themes.is_some()
        && graph.prefabs.is_some()
}

/// Install the GTW-582 reference-integrity pass on the game's `Load` chain:
/// the seam plumbing ([`ContentValidationAppExt::init_content_validation`]),
/// the `Check`-set window (in `Load`, every graph resource resolved, not yet
/// checked), and one [`register_reference_check`] hook per content-graph edge
/// (gate directive P1 — edge/family N+1 is ONE more hook here, never a shared
/// walker edit).
///
/// Registered UNCONDITIONALLY (no `AssetServer` self-gate): under a headless
/// `MinimalPlugins` harness the seeded gate resources open the window, the
/// checks walk the (empty, seeded) graph, and the publish stamps
/// [`ContentValidationDone`](gdtf_assets::ContentValidationDone) — which the
/// `transition_to_intro` gate now requires, so the pass is explicitly ordered
/// before the `Load → Intro` transition on EVERY path.
pub(in crate::states::load) fn add_content_validation(app: &mut App) {
    app.init_content_validation();
    app.configure_sets(
        Update,
        ContentValidationSet::Check.run_if(
            in_state(AppState::Load)
                .and_then(not(resource_exists::<ContentChecksComplete>))
                .and_then(reference_graph_ready),
        ),
    );
    app.register_reference_check(situation::check_situation_gang_refs)
        .register_reference_check(situation::check_situation_theme_ref)
        .register_reference_check(situation::check_situation_terrain_refs)
        .register_reference_check(situation::check_situation_field_refs)
        // The six HOST-AGNOSTIC edge checks, shared with the content editor
        // via gdtf_content_families::validate (GTW-630; the injuries edge
        // joined the shared set in GTW-654 when the editor started loading
        // the injuries family; the terrain graphic_name edge in GTW-663).
        .register_reference_check(check_gang_equipment_refs)
        .register_reference_check(check_weapon_attachment_refs)
        .register_reference_check(check_theme_terrain_refs)
        .register_reference_check(check_emplacement_weapon_refs)
        .register_reference_check(check_injury_weighting_refs)
        .register_reference_check(check_terrain_graphic_refs)
        .register_reference_check(prefabs::check_prefab_refs);
}
