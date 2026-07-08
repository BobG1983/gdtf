//! GTW-575 headless integration test: the editor's nineteen `Editing`-scoped MODEL
//! resources (the GTW-636 `GangDraft`, the GTW-479 `ArmorDraft`, the GTW-654
//! `InjuryDraft` + `WeightingDraft`, the GTW-664 `SpriteDraft`, the GTW-669
//! `AttachmentDraft`, the GTW-670 `WeaponDraft`, the GTW-671 `MeleeWeaponDraft`, and
//! the GTW-594 `IsolateView` included) ride the shared `gdtf_state_scoped` seam with
//! their EXACT lifecycle — absent in `Load`, inserted `OnEnter(Editing)` with the
//! registered seed values, and removed `OnExit(Editing)`.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI
//! harness (the `egui_shell.rs` recipe): the editor's actual `Load` pass
//! resolves the shipped theme + registries, so the `Load → Editing` transition
//! and the scoped inserts are the production path, not a copy.
//!
//! Dir-form suite (module-layout rule 5 — split at the GTW-670 growth point): the
//! harness + the lifecycle test live here; the three roster ASSERT helpers (which grow
//! a block per scoped resource) live in [`asserts`].

mod asserts;

use bevy::prelude::*;
use gdtf_content_editor::{EditorMode, EditorState, MapEditorPlugin};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

use crate::asserts::{assert_all_scoped_resources_absent, assert_all_scoped_resources_seeded};

/// A generous frame cap: the async asset loads under parallel `cargo`
/// contention take a non-deterministic number of frames, so this is a SAFETY
/// NET (not a timing budget) — we poll the `EditorState::Editing` SIGNAL.
const MAX_UPDATES: u32 = 10_000;

/// Builds the real editor app on the no-renderer `DefaultPlugins` UI harness
/// (the `egui_shell.rs` recipe — a live `AssetServer` rooted at the workspace
/// `assets/`, minus the windowed `EguiPlugin` the headless harness cannot
/// host).
fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    // The no-GPU harness (`backends: None`) runs `RenderPlugin::build` — which
    // registers bevy_render's component-sync `on_remove` HOOKS on every camera
    // — but never creates the render app, so the main-world
    // `PendingSyncEntity` resource those hooks push into is missing and ANY
    // camera despawn panics. This test is the first to drive
    // `OnExit(Editing)` headlessly (the editor's preview-camera teardown), so
    // it adds `SyncWorldPlugin`, whose `build` inits exactly that main-world
    // resource; with no render world the sync records simply accumulate,
    // harmlessly. bevy_render only adds the plugin itself when the render app
    // is created, so this add is deterministic under `backends: None` (no
    // double-add).
    app.add_plugins(bevy::render::sync_world::SyncWorldPlugin);
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drives the app until [`EditorState::Editing`], then a few settle frames so
/// the `OnEnter(Editing)` command flushes apply before the assertions read.
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
        "the editor never reached EditorState::Editing — its Load pass did not resolve the \
         theme + registries (a genuine load failure, not a frame-budget shortfall)",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// The full scoped lifecycle: absent while loading, inserted on
/// `OnEnter(Editing)` with the exact seed values, removed on
/// `OnExit(Editing)`.
#[test]
fn editing_scoped_resources_seed_on_enter_and_remove_on_exit() {
    let mut app = editor_app();

    // BEFORE ENTER: the editor boots into `Load`; none of the Editing-scoped
    // model resources may exist yet.
    app.update();
    assert_eq!(
        app.world()
            .get_resource::<State<EditorState>>()
            .map(|s| s.get().clone()),
        Some(EditorState::Load),
        "the editor boots into its Load pass",
    );
    assert_all_scoped_resources_absent(&app, "must be absent while the editor is still loading");

    // IN-STATE: every resource carries its exact seed.
    advance_to_editing(&mut app);
    assert_all_scoped_resources_seeded(&app);

    // AFTER EXIT: queue `Editing → Load`; the `OnExit(Editing)` removes drop
    // every scoped resource. Assert after exactly ONE update: the editor's
    // Load pass auto-advances straight back into Editing (its gate registries
    // are still resolved), so a later read would already see the RE-SEEDED
    // resources of the second span.
    app.world_mut()
        .resource_mut::<NextState<EditorState>>()
        .set(EditorState::Load);
    app.update();
    assert_all_scoped_resources_absent(&app, "must be removed once Editing exits");

    // RE-ENTRY: the second Editing span seeds afresh through the same
    // registrations (the per-span lifetime the hand-stamped pairs had).
    advance_to_editing(&mut app);
    assert_eq!(
        app.world().get_resource::<EditorMode>(),
        Some(&EditorMode::default()),
        "a re-entered Editing span must re-seed its scoped resources",
    );
}
