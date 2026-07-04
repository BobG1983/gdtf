//! GTW-575 headless integration test: the editor's ten `Editing`-scoped MODEL
//! resources ride the shared `gdtf_state_scoped` seam with their EXACT
//! pre-sweep lifecycle — absent in `Load`, inserted `OnEnter(Editing)` with
//! the same seed values the hand-stamped `editor_resources.rs` pairs used, and
//! removed `OnExit(Editing)`.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI
//! harness (the `egui_shell.rs` recipe): the editor's actual `Load` pass
//! resolves the shipped theme + registries, so the `Load → Editing` transition
//! and the scoped inserts are the production path, not a copy.

use bevy::prelude::*;
use gdtf_battle_presenter::ViewMode;
use gdtf_content_editor::{
    CanvasZoom, CurrentEditLevel, EditorMap, EditorMode, EditorState, HoveredCell, MapEditorPlugin,
    MapEditorSession, PreviewPan, TerrainDraft, ThemeDraft,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

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

/// Asserts every one of the ten `Editing`-scoped model resources is absent.
fn assert_all_scoped_resources_absent(app: &App, when: &str) {
    let world = app.world();
    assert!(
        world.get_resource::<EditorMode>().is_none(),
        "EditorMode {when}"
    );
    assert!(
        world.get_resource::<MapEditorSession>().is_none(),
        "MapEditorSession {when}",
    );
    assert!(
        world.get_resource::<EditorMap>().is_none(),
        "EditorMap {when}"
    );
    assert!(
        world.get_resource::<CurrentEditLevel>().is_none(),
        "CurrentEditLevel {when}",
    );
    assert!(
        world.get_resource::<CanvasZoom>().is_none(),
        "CanvasZoom {when}"
    );
    assert!(
        world.get_resource::<TerrainDraft>().is_none(),
        "TerrainDraft {when}"
    );
    assert!(
        world.get_resource::<ThemeDraft>().is_none(),
        "ThemeDraft {when}"
    );
    assert!(
        world.get_resource::<HoveredCell>().is_none(),
        "HoveredCell {when}"
    );
    assert!(
        world.get_resource::<PreviewPan>().is_none(),
        "PreviewPan {when}"
    );
    assert!(
        world.get_resource::<ViewMode>().is_none(),
        "ViewMode {when}"
    );
}

/// Asserts every scoped resource is present with the seed its
/// `init_state_scoped_resource` registration captured — the IDENTICAL
/// constructors the hand-stamped `editor_resources.rs` inserts used.
fn assert_all_scoped_resources_seeded(app: &App) {
    let world = app.world();
    assert_eq!(
        world.get_resource::<EditorMode>(),
        Some(&EditorMode::default()),
        "EditorMode seeds to the default Prefab mode (GTW-474)",
    );
    assert_eq!(
        world.get_resource::<EditorMap>(),
        Some(&EditorMap::new()),
        "EditorMap seeds empty (GTW-426)",
    );
    assert_eq!(
        world.get_resource::<CurrentEditLevel>(),
        Some(&CurrentEditLevel::ground()),
        "CurrentEditLevel seeds to the ground storey (GTW-500 C1)",
    );
    assert_eq!(
        world.get_resource::<CanvasZoom>(),
        Some(&CanvasZoom::identity()),
        "CanvasZoom seeds to the unzoomed identity (GTW-500 C3)",
    );
    assert_eq!(
        world.get_resource::<TerrainDraft>(),
        Some(&TerrainDraft::default()),
        "TerrainDraft seeds to a fresh default draft (GTW-474)",
    );
    assert_eq!(
        world.get_resource::<HoveredCell>(),
        Some(&HoveredCell::new()),
        "HoveredCell seeds empty — nothing hovered (GTW-512 C1.5)",
    );
    assert_eq!(
        world.get_resource::<PreviewPan>(),
        Some(&PreviewPan::origin()),
        "PreviewPan seeds to the origin (GTW-515 C4.8)",
    );
    assert_eq!(
        world.get_resource::<ViewMode>(),
        Some(&ViewMode::default()),
        "ViewMode seeds to the default DownToActive (GTW-532)",
    );
    // ThemeDraft's seed (`ThemeDraft::default` -> `new_theme`) MINTS a fresh
    // `ThemeUuid` per entry by design (GTW-475 C4), so whole-value equality
    // against another fresh default would fail on the key; assert the seeded
    // form state field by field instead.
    // `let … else` keeps the test panic-free per the workspace lints (the
    // preceding assert is what fails the test on absence).
    let theme_draft = world.get_resource::<ThemeDraft>();
    assert!(
        theme_draft.is_some(),
        "ThemeDraft must be present in Editing"
    );
    let Some(theme_draft) = theme_draft else {
        return;
    };
    assert_eq!(
        theme_draft.display_name(),
        "",
        "ThemeDraft seeds with an empty display name (a fresh NEW-theme draft)",
    );
    assert!(
        theme_draft.terrain().is_empty(),
        "ThemeDraft seeds with an empty terrain palette",
    );
    assert_eq!(
        theme_draft.default_floor(),
        None,
        "ThemeDraft seeds with no default floor chosen",
    );
    // MapEditorSession seeds to `MapEditorSession::default()` (nil theme, no
    // floor, the full 60×60×8 grid, no paint tile). The pre-existing
    // `seed_default_theme` drive (GTW-421) may already have re-seeded the
    // theme/floor PAIR from the resolved registry by the time we read — that
    // is unchanged production behavior, so assert the drive-untouched seed
    // fields exactly.
    let session = world.get_resource::<MapEditorSession>();
    assert!(
        session.is_some(),
        "MapEditorSession must be present in Editing"
    );
    let Some(session) = session else {
        return;
    };
    assert_eq!(
        session.grid_size(),
        MapEditorSession::default().grid_size(),
        "MapEditorSession seeds with the full default grid",
    );
    assert_eq!(
        session.selected_tile(),
        None,
        "MapEditorSession seeds with no paint tile selected",
    );
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
