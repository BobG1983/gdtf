use bevy::prelude::*;
use gdtf_assets::ContentFamilyAppExt;
use gdtf_content_families::{
    ArmorFamily, AttachmentsFamily, FieldsFamily, GangsFamily, MeleeWeaponsFamily,
    SpriteDefsFamily, TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily,
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

    register_injuries(app);

    app.add_systems(
        Update,
        transition_to_editing.run_if(in_state(EditorState::Load)),
    );
}
