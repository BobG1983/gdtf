//! Headless integration test for the GTW-464 size-field session → fields reverse sync.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (the GTW-512
//! pattern: headless tests assert the MODEL resources; the egui DRAW itself needs a primary egui
//! context + a window, so it is Screenshot-QA-covered, not headless-covered). The right-panel
//! W / H / Levels fields render from [`SizeFieldSpans::from_session`] — re-derived on EVERY egui
//! pass — so asserting THAT model against the live state-scoped [`MapEditorSession`] proves the
//! panel's render source reflects a programmatic grid change (GTW-464 C1/C3).
//!
//! The IN-FOCUS edit buffer is NOT modelled crate-side: while a `DragValue` has keyboard focus,
//! egui displays its own temp edit `String` (egui memory, keyed by widget id — egui 0.35
//! `drag_value.rs`) and ignores the fed-in value, so a mid-edit programmatic change cannot clobber
//! the user's in-progress text (GTW-464 C2). With no crate-side buffer to assert, the headless
//! test covers the modelled state only (the contract's "if that state is modelled" carve-out).
//!
//! `assert!` + `let … else` keep the test panic-free per the workspace lints (no `unwrap` /
//! `expect` / `panic!`).

use bevy::prelude::*;
use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};
use gdtf_content_editor::{
    CurrentEditLevel, EditorState, GridSpanInput, LevelStep, MapEditorPlugin, MapEditorSession,
    SizeFieldSpans,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

/// A generous frame cap: async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET — we poll a SIGNAL, not a count.
const MAX_UPDATES: u32 = 10_000;

/// Build the real editor app on the no-renderer `DefaultPlugins` UI harness (the SAME plugin the
/// binary wires, minus the windowed `EguiPlugin` the headless harness has no window for — the
/// MODEL resources the size fields render from still run their full lifecycle).
fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drive the app to `Editing`, then a few frames so the `OnEnter(Editing)` inserts apply.
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
        "the editor never reached EditorState::Editing — its Load pass did not resolve the theme + \
         registries",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// GTW-464 C1 / C3 (REAL PATH) — a PROGRAMMATIC grid change through the live state-scoped
/// [`MapEditorSession`] (the exact [`MapEditorSession::set_grid_size`] call the dev-capture
/// drive makes, and the loaded-prefab path will make per GTW-433) is reflected in
/// [`SizeFieldSpans::from_session`] — the model the right-panel W / H / Levels fields render from
/// on every egui pass. No stale seeded spans survive the change.
#[test]
fn programmatic_grid_change_updates_the_displayed_spans() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    // The seeded session displays its own grid size (whatever the seed is — no magnitude pin).
    {
        let world = app.world();
        let Some(session) = world.get_resource::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        let displayed = SizeFieldSpans::from_session(session);
        assert_eq!(
            *displayed.width(),
            *session.grid_size().width(),
            "the displayed width mirrors the seeded session",
        );
    }

    // Programmatically shrink the grid through the REAL session resource — the same call the
    // dev-capture `drive_capture_grid_size` makes (and GTW-433's situation/prefab load will make).
    let Ok(shrunk) = GridSize::new(GridWidth::new(16), GridHeight::new(16), GridLevels::new(1))
    else {
        unreachable!("a 16×16×1 grid is valid");
    };
    {
        let world = app.world_mut();
        let Some(mut session) = world.get_resource_mut::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        session.set_grid_size(shrunk);
    }
    // A frame passes (the egui pass would re-derive the spans on its next run).
    app.update();

    // The model the panel renders from now shows the NEW size — not the stale seed.
    let world = app.world();
    let Some(session) = world.get_resource::<MapEditorSession>() else {
        unreachable!("session inserted in Editing");
    };
    let displayed = SizeFieldSpans::from_session(session);
    assert_eq!(
        *displayed.width(),
        16,
        "the displayed width reflects the programmatic change (C1)"
    );
    assert_eq!(
        *displayed.height(),
        16,
        "the displayed height reflects the programmatic change (C1)"
    );
    assert_eq!(
        *displayed.levels(),
        1,
        "the displayed levels reflect the programmatic change (C1)"
    );
}

/// GTW-464 C1 (the KEPT forward path) — an edited-span [`SizeFieldSpans::commit`] still folds
/// fields → session through the kept clamp, and re-clamps the live [`CurrentEditLevel`] so a
/// shrunk volume never leaves the edit cursor past the new extent (the kept level-nav clamp).
#[test]
fn commit_preserves_the_field_to_session_path_and_reclamps_the_edit_level() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    // Grow to a 3-storey volume and lift the edit cursor to the top storey (2).
    let Ok(tall) = GridSize::new(GridWidth::new(6), GridHeight::new(6), GridLevels::new(3)) else {
        unreachable!("a 6×6×3 grid is valid");
    };
    {
        let world = app.world_mut();
        let Some(mut session) = world.get_resource_mut::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        session.set_grid_size(tall);
        let Some(mut edit_level) = world.get_resource_mut::<CurrentEditLevel>() else {
            unreachable!("edit level inserted in Editing");
        };
        *edit_level = CurrentEditLevel::ground()
            .stepped(LevelStep::up(), tall)
            .stepped(LevelStep::up(), tall);
        assert_eq!(*edit_level.level(), 2, "the edit cursor sits on storey 2");
    }

    // Commit a user-edit that shrinks the volume to ONE storey — the exact spans a user typing
    // `1` into the Levels DragValue produces, folded by the same commit the `.changed()` response
    // fires — against the LIVE session + edit-level resources.
    {
        let edited = SizeFieldSpans::new(
            GridSpanInput::new(6),
            GridSpanInput::new(6),
            GridSpanInput::new(1),
        );
        let world = app.world_mut();
        let Some(mut session) = world.get_resource_mut::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        // `commit` takes the session + edit level mutably; the level travels through a scratch
        // (seeded to the same storey-2 cursor set above) because two `ResMut`s cannot be borrowed
        // from the world at once here.
        let mut edit_level = CurrentEditLevel::ground()
            .stepped(LevelStep::up(), tall)
            .stepped(LevelStep::up(), tall);
        edited.commit(&mut session, &mut edit_level);
        assert_eq!(
            *session.grid_size().levels(),
            1,
            "the commit folds the edited spans into the session (the kept forward path)",
        );
        assert_eq!(
            *edit_level.level(),
            0,
            "the commit re-clamps an out-of-range edit cursor into the shrunk volume",
        );
    }
}
