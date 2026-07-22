//! GTW-479 C3/A2: the ARMOR mode's REAL round-trip — author an armor suit in the form
//! model, save it through the REAL root-parameterized write (`write_armor_in`) into a
//! `TempDir` assets root (the GTW-555 pattern — the shipped `assets/` tree is NEVER
//! written), then boot the REAL editor app rooted at that directory and assert the
//! actual `ArmorFamily` folder walk loads the saved suit back structurally identical.
//!
//! Also pins the GTW-479 lifecycle riders: the editor reaches `Editing` with the
//! `ArmorRegistry` gate resource present and the state-scoped `ArmorDraft` seeded
//! (salvage / fallback behavior itself is the shared registration's parameterized family contract —
//! `register_content_family::<ArmorFamily>` inherits it, no per-family re-pin here).

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
use gdtf_battle_sim::armor::{
    ArmorHardness, ArmorIntegrity, ArmorName, ArmorProtection, ArmorRegistry, ArmorType, BodyPart,
};
use gdtf_content_editor::{
    ArmorDraft, EditorState, MapEditorPlugin, draft_to_spec, write_armor_in,
};
use gdtf_test_utils::advance_until;

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET (not a timing budget) —
/// the test polls the `EditorState::Editing` SIGNAL.
const MAX_UPDATES: u32 = 10_000;

/// The real editor app rooted at an ARBITRARY assets directory (the `load_seam.rs`
/// failure-path recipe): only `content/armor/` is materialized by this test, so every
/// other family fails closed to its empty registry (the no-strand guarantee) while the
/// armor walk loads the REAL saved file.
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
        "the editor never reached EditorState::Editing — the Load gate (including the \
         ArmorRegistry) did not resolve or fall back",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// The edited armor the test authors through the REAL form-model mutators: a renamed
/// suit with per-part-distinct edits (so a swapped body-part slot cannot round-trip).
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

/// GTW-479 — create → save (the REAL write into a `TempDir` assets root) → load through
/// the REAL `ArmorFamily` folder walk → the registry holds the SAME spec (structural
/// equality across all six per-part pieces), with the `Editing` gate + the scoped
/// `ArmorDraft` seed along for the ride.
#[test]
fn saved_armor_round_trips_through_the_real_armor_family_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // SAVE through the real root-parameterized write.
    let draft = edited_draft();
    let (name, spec) = draft_to_spec(&draft);
    let written = write_armor_in(dir.path(), &name, &spec);
    assert!(
        written.is_ok(),
        "the real armor write must succeed: {:?}",
        written.as_ref().err(),
    );

    // RELOAD through the real editor Load pass rooted at the TempDir.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    // The state-scoped ARMOR draft seeded on entering Editing (bevy-traps #1 via the
    // GTW-575 shared registration).
    assert!(
        world.get_resource::<ArmorDraft>().is_some(),
        "the ArmorDraft must be seeded OnEnter(Editing)",
    );

    // The REAL folder walk keyed the saved file by its stem and loaded the SAME spec.
    let registry = world.get_resource::<ArmorRegistry>();
    assert!(registry.is_some(), "the ArmorRegistry must resolve");
    let Some(registry) = registry else { return };
    let reloaded = registry.spec(&ArmorName::new("tempdir_plate".to_owned()));
    assert_eq!(
        reloaded,
        Some(&spec),
        "the reloaded armor must equal the saved spec (all six per-part pieces: floor / \
         protection / integrity / hardness / armor_type) — the GTW-269 stem-key round-trip \
         through the REAL loader",
    );
}
