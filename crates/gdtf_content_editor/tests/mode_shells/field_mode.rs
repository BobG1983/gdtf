//! The FIELD mode's REAL round-trip. Author a field def in the form's own draft,
//! write it with the form's own writer, and read it back through the real fields loader.
use std::{num::NonZeroU8, path::Path};

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_sim::{
    armor::ArmorType,
    effects::fields::{
        FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldTurns,
        ImmuneArmorTypes,
    },
    weapon::DamageType,
};
use gdtf_content_editor::{
    EditorState, FieldDraft, MapEditorPlugin, draft_to_field, write_field_in,
};
use gdtf_test_utils::advance_until;

fn editor_app_with_asset_root(root: &Path) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            }),
    );
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

fn advance_to_editing(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|s| *s.get() == EditorState::Editing)
    });
    for _ in 0..4 {
        app.update();
    }
}

fn edited_draft() -> FieldDraft {
    let Some(count) = NonZeroU8::new(2) else {
        unreachable!("2 is not zero");
    };
    let mut draft = FieldDraft::new_field();
    draft.set_key("tempdir_sump".to_owned());
    draft.set_damage(FieldDamage::new(5));
    draft.set_damage_type(DamageType::Chem);
    draft.set_duration(FieldDuration::Turns(FieldTurns::new(count)));
    draft.toggle_immune_armor_type(ArmorType::Hazard);
    draft.toggle_immune_armor_type(ArmorType::Flak);
    draft
}

#[test]
fn saved_field_round_trips_through_the_real_fields_family_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let draft = edited_draft();
    let (key, def) = draft_to_field(&draft);
    let written = write_field_in(dir.path(), &key, &def);
    assert!(
        written.is_ok(),
        "the real field write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<FieldDraft>().is_some(),
        "the FieldDraft must be seeded OnEnter(Editing)",
    );

    let registry = world.get_resource::<FieldDefRegistry>();
    assert!(
        registry.is_some(),
        "the FieldDefRegistry must resolve, which it only does once the fields family is \
         registered in the editor's own load pass",
    );
    let Some(registry) = registry else { return };
    let reloaded = registry.def(&FieldKey::new("tempdir_sump".to_owned()));
    assert_eq!(
        reloaded,
        Some(&def),
        "the reloaded field must equal the saved def (damage / channel / immune list / \
         duration). The stem key round-trips through the REAL loader",
    );
    let Some(reloaded) = reloaded else { return };
    assert_eq!(
        reloaded.immune_armor_types,
        ImmuneArmorTypes::new([ArmorType::Flak, ArmorType::Hazard]),
        "both toggled armor types survive the write and the load",
    );
    assert_eq!(
        FieldDef::new(
            reloaded.damage,
            reloaded.damage_type,
            reloaded.immune_armor_types.clone(),
            reloaded.duration,
        ),
        def,
        "every one of the four authored values is what the loader read back",
    );
}
