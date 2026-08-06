use bevy::{math::Vec2, prelude::*, window::PrimaryWindow};
use gdtf_battle_input::world_to_cell;
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::prelude::Level;

use super::harness::*;

#[test]
fn picking_resolves_the_cursor_to_the_documented_cell() {
    let level = Level::new(0);
    let mut app = picking_app(level);

    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    app.update();

    let world = unproject(&mut app, cursor);
    let Some(world) = world else {
        unreachable!("the synthetic camera must unproject the in-grid cursor");
    };
    let expected = world_to_cell(world, level);
    assert!(
        expected.is_some(),
        "the chosen cursor must land inside the grid (world {world:?})",
    );

    assert_eq!(
        hovered(&app),
        expected,
        "InspectTarget must equal the documented inverse of the camera unprojection",
    );
}

#[test]
fn picking_fails_closed_to_none() {
    let level = Level::new(0);

    {
        let mut app = picking_app(level);
        set_cursor(&mut app, None);
        app.update();
        assert_eq!(
            hovered(&app),
            None,
            "an off-window cursor (no cursor_position) must resolve InspectTarget to None",
        );
    }

    {
        let mut app = picking_app(level);
        let cursor = TARGET_SIZE * 0.5 - Vec2::new(64.0, 0.0);
        set_cursor(&mut app, Some(cursor));
        app.update();
        let world = unproject(&mut app, cursor);
        let Some(world) = world else {
            unreachable!("the synthetic camera must unproject the left-of-origin cursor");
        };
        assert_eq!(
            world_to_cell(world, level),
            None,
            "the chosen left-of-origin cursor must be off-grid (world {world:?})",
        );
        assert_eq!(
            hovered(&app),
            None,
            "an off-grid cursor must resolve InspectTarget to None",
        );
    }

    {
        let mut app = picking_app(level);
        let cameras: Vec<Entity> = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<WorldCamera>>();
            q.iter(app.world()).collect()
        };
        for camera in cameras {
            app.world_mut().entity_mut(camera).despawn();
        }
        set_cursor(&mut app, Some(TARGET_SIZE * 0.5));
        app.update();
        assert_eq!(
            hovered(&app),
            None,
            "with no WorldCamera the picking must resolve InspectTarget to None",
        );
    }

    {
        let mut app = picking_app(level);
        set_cursor(&mut app, Some(TARGET_SIZE * 0.5));
        app.update();
        assert!(
            hovered(&app).is_some(),
            "the centre cursor must resolve a cell while the window is there, or despawning it \
             below proves nothing",
        );

        let windows: Vec<Entity> = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<PrimaryWindow>>();
            q.iter(app.world()).collect()
        };
        for window in windows {
            app.world_mut().entity_mut(window).despawn();
        }
        app.update();
        assert_eq!(
            hovered(&app),
            None,
            "with no primary window the picking must resolve InspectTarget to None, which is \
             what every windowless QA host relies on",
        );
    }
}
