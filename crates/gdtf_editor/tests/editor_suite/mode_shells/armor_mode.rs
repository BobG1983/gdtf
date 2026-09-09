//! C3/A2: the ARMOR mode's REAL round-trip — author an armor suit in the form
use std::path::Path;

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::{advance_until, asset_plugin_at};
use gdtf_battle_sim::armor::{
    ArmorHardness, ArmorIntegrity, ArmorName, ArmorProtection, ArmorRegistry, ArmorType, BodyPart,
};
use gdtf_editor::{ArmorDraft, EditorState, MapEditorPlugin, draft_to_spec, write_armor_in};

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
            .set(asset_plugin_at(root)),
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

fn edited_draft() -> ArmorDraft {
    let mut draft = ArmorDraft::new_armor();
    draft.set_name("tempdir_plate".to_owned());
    draft.piece_mut(BodyPart::Head).armor_type = ArmorType::Ceramic;
    draft.piece_mut(BodyPart::Torso).protection = ArmorProtection::new(5);
    draft.piece_mut(BodyPart::Torso).integrity = ArmorIntegrity::new(60);
    draft.piece_mut(BodyPart::LeftArm).hardness = ArmorHardness::new(2);
    draft.piece_mut(BodyPart::RightLeg).armor_type = ArmorType::Hazard;
    draft
}

#[test]
fn saved_armor_round_trips_through_the_real_armor_family_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let draft = edited_draft();
    let (name, spec) = draft_to_spec(&draft);
    let written = write_armor_in(dir.path(), &name, &spec);
    assert!(
        written.is_ok(),
        "the real armor write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<ArmorDraft>().is_some(),
        "the ArmorDraft must be seeded OnEnter(Editing)",
    );

    let registry = world.get_resource::<ArmorRegistry>();
    assert!(registry.is_some(), "the ArmorRegistry must resolve");
    let Some(registry) = registry else { return };
    let reloaded = registry.spec(&ArmorName::new("tempdir_plate".to_owned()));
    assert_eq!(
        reloaded,
        Some(&spec),
        "the reloaded armor must equal the saved spec (all six per-part pieces: floor / \
         protection / integrity / hardness / armor_type) — the stem-key round-trip \
         through the REAL loader",
    );
}
