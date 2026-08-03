use bevy::{
    prelude::*,
    ui::{BackgroundColor, Node, widget::Button},
};

use super::{
    super::{ThemeRole, Themed},
    support::{app_with_production_run_condition, theme},
};

#[test]
fn incremental_repaint_does_not_recolor_existing_on_new_spawn()
-> Result<(), ron::error::SpannedError> {
    let mut app = app_with_production_run_condition();
    let base = Color::srgb(0.12, 0.12, 0.15);
    app.insert_resource(theme(
        [0.12, 0.12, 0.15],
        [0.20, 0.20, 0.24],
        2.0,
        5.0,
        18.0,
    )?);

    let e = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Button, Node::default()))
        .id();
    app.update();
    app.update();

    let sentinel = Color::srgb(0.80, 0.16, 0.19);
    if let Some(mut bg) = app.world_mut().get_mut::<BackgroundColor>(e) {
        bg.0 = sentinel;
    }

    let f = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Button, Node::default()))
        .id();
    app.update();

    let world = app.world();
    assert_eq!(
        world.get::<BackgroundColor>(f).map(|c| c.0),
        Some(base),
        "a newly-spawned Themed widget must get its initial base paint",
    );
    assert_eq!(
        world.get::<BackgroundColor>(e).map(|c| c.0),
        Some(sentinel),
        "an existing widget's fill must NOT be recolored to base by an unrelated Themed spawn",
    );

    Ok(())
}

#[test]
fn theme_change_repaints_all_themed() -> Result<(), ron::error::SpannedError> {
    let mut app = app_with_production_run_condition();
    app.insert_resource(theme(
        [0.12, 0.12, 0.15],
        [0.20, 0.20, 0.24],
        2.0,
        5.0,
        18.0,
    )?);

    let e = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Button, Node::default()))
        .id();
    app.update();
    app.update();

    let new_base = Color::srgb(0.50, 0.10, 0.30);
    app.insert_resource(theme(
        [0.50, 0.10, 0.30],
        [0.99, 0.40, 0.00],
        4.0,
        9.0,
        30.0,
    )?);
    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(e).map(|c| c.0),
        Some(new_base),
        "a real theme change must repaint EVERY existing Themed widget to the new base",
    );

    Ok(())
}
