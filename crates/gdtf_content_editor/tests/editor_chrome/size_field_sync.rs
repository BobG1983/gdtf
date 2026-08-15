//! Size field sync: grid changes update spans; commit reclamps the edit level.
use bevy::prelude::*;
use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};
use gdtf_content_editor::{
    CurrentEditLevel, EditorState, GridSpanInput, LevelStep, MapEditorPlugin, MapEditorSession,
    SizeFieldSpans,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
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

#[test]
fn programmatic_grid_change_updates_the_displayed_spans() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

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
    app.update();

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

#[test]
fn commit_preserves_the_field_to_session_path_and_reclamps_the_edit_level() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

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
