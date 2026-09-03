use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::{ContentFamilyAppExt, ContentValidationDone, HotRonAppExt, RonAssetAppExt};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    effects::fields::FieldDefRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::{InjuryDef, InjuryRegistry, InjuryWeighting},
    level::{PrefabRegistry, PrefabSpec, UuidThemeRegistry},
    procgen::ProcgenTuning,
    situation::Situation,
    terrain::def::TerrainDefRegistry,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::{
    ArmorFamily, AttachmentsFamily, FieldsFamily, GangsFamily, MeleeWeaponsFamily,
    SpriteDefsFamily, TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily,
    injuries::{INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION},
    prefabs::PREFAB_EXTENSION,
    situation::{
        LoadedSituation, SITUATION_RON_PATH, fallback_loaded_situation, map_loaded_situation,
    },
    sprites::SpriteDefRegistry,
};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

use crate::states::{
    AppState,
    load::{resources::LoadHandles, systems::*},
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit},
};

const TUNING_RON_PATH: &str = "core_tuning/combat.tuning.ron";

const STAT_TUNING_RON_PATH: &str = "core_tuning/stat.tuning.ron";

const PROCGEN_TUNING_RON_PATH: &str = "core_tuning/procgen.tuning.ron";

pub(in crate::states) struct LoadScenePlugin;

impl Plugin for LoadScenePlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<AssetServer>().is_some() {
            app.init_ron_asset::<GdtfThemeSpec>();
            app.init_hot_ron_resource_mapped_with_fallback::<Situation, LoadedSituation>(
                SITUATION_RON_PATH,
                map_loaded_situation,
                fallback_loaded_situation,
            );
            app.init_hot_ron_resource_with_fallback::<CombatTuning>(
                TUNING_RON_PATH,
                CombatTuning::default,
            );
            app.init_hot_ron_resource_with_fallback::<GangerStatTuning>(
                STAT_TUNING_RON_PATH,
                GangerStatTuning::default,
            );
            app.init_hot_ron_resource_with_fallback::<ProcgenTuning>(
                PROCGEN_TUNING_RON_PATH,
                ProcgenTuning::default,
            );
            app.init_ron_asset_with_extensions::<InjuryDef>(vec![INJURY_DEF_EXTENSION]);
            app.init_ron_asset_with_extensions::<InjuryWeighting>(vec![INJURY_WEIGHTING_EXTENSION]);
            app.init_ron_asset_with_extensions::<PrefabSpec>(vec![PREFAB_EXTENSION]);
            add_hot_reload_systems(app);
        }
        app.register_content_family::<WeaponsFamily>();
        app.register_content_family::<MeleeWeaponsFamily>();
        app.register_content_family::<ArmorFamily>();
        app.register_content_family::<FieldsFamily>();
        app.register_content_family::<GangsFamily>();
        app.register_content_family::<TerrainDefsFamily>();
        app.register_content_family::<ThemeDefsFamily>();
        app.register_content_family::<AttachmentsFamily>();
        app.register_content_family::<SpriteDefsFamily>();
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Load");
    add_content_validation(app);
    app.add_systems(
        OnEnter(AppState::Load),
        (log_scene_enter(label), kick_off_loads).chain(),
    )
    .add_systems(
        Update,
        (
            poll_and_resolve.run_if(
                in_state(AppState::Load)
                    .and_then(resource_exists::<LoadHandles>)
                    .and_then(
                        not(resource_exists::<GdtfTheme>)
                            .or_else(not(resource_exists::<InjuryRegistry>))
                            .or_else(not(resource_exists::<PrefabRegistry>)),
                    ),
            ),
            transition_to_intro.run_if(
                in_state(AppState::Load)
                    .and_then(resource_exists::<GdtfTheme>)
                    .and_then(resource_exists::<CombatTuning>)
                    .and_then(resource_exists::<GangerStatTuning>)
                    .and_then(resource_exists::<ProcgenTuning>)
                    .and_then(resource_exists::<WeaponRegistry>)
                    .and_then(resource_exists::<MeleeWeaponRegistry>)
                    .and_then(resource_exists::<AttachmentRegistry>)
                    .and_then(resource_exists::<LoadedSituation>)
                    .and_then(resource_exists::<ArmorRegistry>)
                    .and_then(resource_exists::<FieldDefRegistry>)
                    .and_then(resource_exists::<InjuryRegistry>)
                    .and_then(resource_exists::<GangRegistry>)
                    .and_then(resource_exists::<PrefabRegistry>)
                    .and_then(resource_exists::<TerrainDefRegistry>)
                    .and_then(resource_exists::<UuidThemeRegistry>)
                    .and_then(resource_exists::<SpriteDefRegistry>)
                    .and_then(resource_exists::<ContentValidationDone>),
            ),
        )
            .chain(),
    )
    .add_systems(
        OnExit(AppState::Load),
        (log_scene_exit(label), cleanup).chain(),
    );
}

fn add_hot_reload_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            redrive_injuries_on_asset_event,
            redrive_prefabs_on_asset_event,
        ),
    );
}
