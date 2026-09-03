//! authoring-time twin of the game's `add_content_validation`.

use bevy::{
    ecs::schedule::SystemCondition as _,
    prelude::{App, IntoScheduleConfigs, Res, Update, not, resource_exists},
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
    situation::LoadedSituation,
    sprites::SpriteDefRegistry,
    validate::{
        check_emplacement_weapon_refs, check_gang_equipment_refs, check_injury_weighting_refs,
        check_prefab_refs, check_situation_field_refs, check_situation_gang_refs,
        check_situation_terrain_refs, check_situation_theme_ref, check_terrain_leaves_behind_refs,
        check_terrain_view_coverage, check_terrain_view_sprite_refs, check_theme_terrain_refs,
        check_weapon_attachment_refs,
    },
};

use super::rearm::rearm_validation_on_content_change;

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct ValidationGraphResources<'w> {
    weapons:       Option<Res<'w, WeaponRegistry>>,
    melee_weapons: Option<Res<'w, MeleeWeaponRegistry>>,
    armor:         Option<Res<'w, ArmorRegistry>>,
    gangs:         Option<Res<'w, GangRegistry>>,
    terrain:       Option<Res<'w, TerrainDefRegistry>>,
    themes:        Option<Res<'w, UuidThemeRegistry>>,
    injuries:      Option<Res<'w, InjuryRegistry>>,
    sprite_defs:   Option<Res<'w, SpriteDefRegistry>>,
    attachments:   Option<Res<'w, AttachmentRegistry>>,
    prefabs:       Option<Res<'w, PrefabRegistry>>,
    situation:     Option<Res<'w, LoadedSituation>>,
    fields:        Option<Res<'w, FieldDefRegistry>>,
}

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
        && graph.prefabs.is_some()
        && graph.situation.is_some()
        && graph.fields.is_some()
}

pub(crate) fn register_validation(app: &mut App) {
    app.init_content_validation();
    app.configure_sets(
        Update,
        ContentValidationSet::Check
            .run_if(not(resource_exists::<ContentChecksComplete>).and_then(validation_graph_ready)),
    );
    app.register_reference_check(check_theme_terrain_refs)
        .register_reference_check(check_emplacement_weapon_refs)
        .register_reference_check(check_gang_equipment_refs)
        .register_reference_check(check_injury_weighting_refs)
        .register_reference_check(check_terrain_view_coverage)
        .register_reference_check(check_terrain_view_sprite_refs)
        .register_reference_check(check_terrain_leaves_behind_refs)
        .register_reference_check(check_weapon_attachment_refs)
        .register_reference_check(check_prefab_refs)
        .register_reference_check(check_situation_gang_refs)
        .register_reference_check(check_situation_theme_ref)
        .register_reference_check(check_situation_terrain_refs)
        .register_reference_check(check_situation_field_refs);
    app.add_systems(Update, rearm_validation_on_content_change);
}
