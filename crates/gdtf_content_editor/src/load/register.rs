use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::{ContentFamilyAppExt, HotRonAppExt as _};
use gdtf_battle_sim::situation::Situation;
use gdtf_content_families::{
    ArmorFamily, AttachmentsFamily, FieldsFamily, GangsFamily, MeleeWeaponsFamily, PrefabsFamily,
    SpriteDefsFamily, TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily,
    situation::{
        LoadedSituation, SITUATION_RON_PATH, fallback_loaded_situation, map_loaded_situation,
    },
};

use crate::{
    EditorState,
    load::{injuries::register_injuries, transition::transition_to_editing},
};

pub(crate) fn register_load(app: &mut App) {
    app.register_content_family::<WeaponsFamily>();
    app.register_content_family::<ArmorFamily>();
    app.register_content_family::<TerrainDefsFamily>();
    app.register_content_family::<ThemeDefsFamily>();
    app.register_content_family::<GangsFamily>();
    app.register_content_family::<MeleeWeaponsFamily>();
    app.register_content_family::<SpriteDefsFamily>();
    app.register_content_family::<AttachmentsFamily>();
    app.register_content_family::<FieldsFamily>();
    app.register_content_family::<PrefabsFamily>();

    if app.world().get_resource::<AssetServer>().is_some() {
        app.init_hot_ron_resource_mapped_with_fallback::<Situation, LoadedSituation>(
            SITUATION_RON_PATH,
            map_loaded_situation,
            fallback_loaded_situation,
        );
    }

    register_injuries(app);

    app.add_systems(
        Update,
        transition_to_editing.run_if(in_state(EditorState::Load)),
    );
}
