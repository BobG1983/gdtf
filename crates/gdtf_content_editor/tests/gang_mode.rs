//! GTW-636 C5: the GANG mode's REAL round-trip — create a gang in the form model, save
//! it through the REAL root-parameterized write (`write_gang_in`) into a `TempDir`
//! assets root (the GTW-555 pattern — the shipped `assets/` tree is NEVER written), then
//! boot the REAL editor app rooted at that directory and assert the actual
//! `GangsFamily` folder walk loads the saved gang back structurally identical —
//! including the authored `melee_weapon` key the retired in-game editor's model dropped.
//!
//! Also pins the GTW-636 lifecycle riders: the editor reaches `Editing` with the
//! `GangRegistry` + `MeleeWeaponRegistry` gate resources present and the state-scoped
//! `GangDraft` seeded (salvage / fallback behavior itself is the seam's parameterized
//! family contract — `register_content_family::<GangsFamily>` inherits it, no per-family
//! re-pin here).

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

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET (not a timing budget) —
/// the test polls the `EditorState::Editing` SIGNAL.
const MAX_UPDATES: u32 = 10_000;

/// The real editor app rooted at an ARBITRARY assets directory (the `load_seam.rs`
/// failure-path recipe): only `content/gangs/` is materialized by this test, so every
/// other family fails closed to its empty registry (the no-strand guarantee) while the
/// gangs walk loads the REAL saved file.
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
            // Headless-test noise suppression (GTW-139): the deliberate
            // failure-path asset errors of the unmaterialized families stay quiet.
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
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler
    // (default panics); with no render backend some render-provided params cannot
    // validate. `warn` restores the skip-with-a-log behavior (the shared harness
    // precedent).
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drives the app until [`EditorState::Editing`], then a few settle frames so the
/// `OnEnter(Editing)` command flushes apply before the assertions read.
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

/// The edited gang the test authors through the REAL form-model mutators: a renamed
/// gang, two members, per-field edits including an authored melee key.
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
        // The field the retired in-game editor DROPPED on save — pinned surviving.
        first.melee_weapon = Some(WeaponName::new("chainsword".to_owned()));
    }
    if let Some(second) = members.get_mut(1) {
        second.name = GangerName::new("Second Member".to_owned());
        second.weapon = WeaponName::new("autogun".to_owned());
    }
    draft
}

/// GTW-636 C5 — create → save (the REAL write into a `TempDir` assets root) → load
/// through the REAL `GangsFamily` folder walk → the registry holds the SAME roster
/// (structural equality, melee key included), with the `Editing` gate + the scoped
/// `GangDraft` seed along for the ride.
#[test]
fn saved_gang_round_trips_through_the_real_gangs_family_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // SAVE through the real root-parameterized write.
    let draft = edited_draft();
    let (name, roster) = draft_to_roster(&draft);
    let written = write_gang_in(dir.path(), &name, &roster);
    assert!(
        written.is_ok(),
        "the real gang write must succeed: {:?}",
        written.as_ref().err(),
    );

    // RELOAD through the real editor Load pass rooted at the TempDir.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    // The GTW-636 gate riders resolved (gangs loaded; melee fails closed to empty —
    // this root materializes no melee folder).
    let melee = world.get_resource::<MeleeWeaponRegistry>();
    assert!(
        melee.is_some_and(MeleeWeaponRegistry::is_empty),
        "the MeleeWeaponRegistry gate resource must fail closed to EMPTY on this root",
    );
    // The state-scoped GANG draft seeded on entering Editing (bevy-traps #1 via the
    // GTW-575 seam).
    assert!(
        world.get_resource::<GangDraft>().is_some(),
        "the GangDraft must be seeded OnEnter(Editing)",
    );

    // The REAL folder walk keyed the saved file by its stem and loaded the SAME roster.
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
