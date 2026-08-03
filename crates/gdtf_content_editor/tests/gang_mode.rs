//! including the authored `melee_weapon` key the retired in-game editor's model dropped.
use std::path::Path;

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
    armor::ArmorName,
    ganger::{GangName, GangRegistry, GangerName, Toughness},
    weapon::{MeleeWeaponRegistry, WeaponName},
};
use gdtf_content_editor::{
    EditorState, GangDraft, MapEditorPlugin, draft_to_roster, write_gang_in,
};
use gdtf_test_utils::advance_until;

const MAX_UPDATES: u32 = 10_000;

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
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — the Load gate (now including the \
         GTW-636 GangRegistry + MeleeWeaponRegistry) did not resolve or fall back",
    );
    for _ in 0..4 {
        app.update();
    }
}

fn edited_draft() -> GangDraft {
    let mut draft = GangDraft::new_gang();
    draft.set_name("tempdir_gang".to_owned());
    draft.add_member();
    draft.add_member();
    let members = draft.members_mut();
    if let Some(first) = members.first_mut() {
        first.name = GangerName::new("Round Tripper".to_owned());
        first.toughness = Toughness::new(11.5);
        first.weapon = WeaponName::new("stub_pistol".to_owned());
        first.armor = ArmorName::new("flak_vest".to_owned());
        first.melee_weapon = Some(WeaponName::new("chainsword".to_owned()));
    }
    if let Some(second) = members.get_mut(1) {
        second.name = GangerName::new("Second Member".to_owned());
        second.weapon = WeaponName::new("autogun".to_owned());
    }
    draft
}

#[test]
fn saved_gang_round_trips_through_the_real_gangs_family_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let draft = edited_draft();
    let (name, roster) = draft_to_roster(&draft);
    let written = write_gang_in(dir.path(), &name, &roster);
    assert!(
        written.is_ok(),
        "the real gang write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    let melee = world.get_resource::<MeleeWeaponRegistry>();
    assert!(
        melee.is_some_and(MeleeWeaponRegistry::is_empty),
        "the MeleeWeaponRegistry gate resource must fail closed to EMPTY on this root",
    );
    assert!(
        world.get_resource::<GangDraft>().is_some(),
        "the GangDraft must be seeded OnEnter(Editing)",
    );

    let registry = world.get_resource::<GangRegistry>();
    assert!(registry.is_some(), "the GangRegistry must resolve");
    let Some(registry) = registry else { return };
    let reloaded = registry.roster(&GangName::new("tempdir_gang".to_owned()));
    assert_eq!(
        reloaded,
        Some(&roster),
        "the reloaded gang must equal the saved roster (names + attributes + weapon/armor \
         keys + the authored melee_weapon) — the GTW-415 round-trip through the REAL loader",
    );
}
