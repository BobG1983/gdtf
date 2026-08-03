use bevy::{
    MinimalPlugins,
    asset::AssetPlugin,
    input::InputPlugin,
    prelude::*,
    scene::ScenePlugin,
    ui::{BackgroundColor, Interaction},
};

use super::support::{app_with_interaction, set_interaction, theme};
use crate::{
    UiPlugin,
    widgets::core::{ActiveButton, ButtonLabel, DisabledButton, spawn_button},
};

#[test]
fn hot_reload_repaints_resting_and_hovered_from_new_theme() -> Result<(), ron::error::SpannedError>
{
    let old = theme(
        [0.08, 0.08, 0.10, 1.0],
        [0.20, 0.20, 0.24, 1.0],
        [0.04, 0.04, 0.06, 1.0],
    )?;
    let mut app = app_with_interaction();
    app.insert_resource(old.clone());

    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(&mut commands, &old, ButtonLabel::new("Fight"), ())
    };
    app.world_mut().flush();
    app.update();

    let new = theme(
        [0.50, 0.10, 0.30, 1.0],
        [0.90, 0.70, 0.10, 1.0],
        [0.10, 0.40, 0.80, 1.0],
    )?;
    app.insert_resource(new.clone());

    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*new.button.color),
        "resting button must reflect the NEW base after hot-reload",
    );

    set_interaction(&mut app, button, Interaction::Hovered);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*new.button.hover),
        "hovered button must reflect the NEW HoverBg after hot-reload",
    );
    assert_ne!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*old.button.hover),
        "hovered button must NOT show the stale OLD HoverBg",
    );

    Ok(())
}

#[test]
fn held_hover_button_repaints_to_new_hover_on_theme_change() -> Result<(), ron::error::SpannedError>
{
    let old = theme(
        [0.08, 0.08, 0.10, 1.0],
        [0.20, 0.20, 0.24, 1.0],
        [0.04, 0.04, 0.06, 1.0],
    )?;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(ScenePlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);
    app.insert_resource(old.clone());

    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(&mut commands, &old, ButtonLabel::new("Fight"), ())
    };
    app.world_mut().flush();

    app.update();
    app.update();

    set_interaction(&mut app, button, Interaction::Hovered);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*old.button.hover),
        "precondition: the held hover must show the OLD hover_bg before the reload",
    );

    let new = theme(
        [0.50, 0.10, 0.30, 1.0],
        [0.90, 0.70, 0.10, 1.0],
        [0.10, 0.40, 0.80, 1.0],
    )?;
    app.insert_resource(new.clone());
    assert_eq!(
        app.world().get::<Interaction>(button).copied(),
        Some(Interaction::Hovered),
        "precondition: the button's Interaction stays Hovered across the reload",
    );
    assert_ne!(
        *new.button.hover, *new.button.color,
        "test precondition: the NEW hover_bg and resting base must differ",
    );

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*new.button.hover),
        "a held-hover button must repaint to the NEW hover_bg the same frame the theme \
         changes (only repaint_theme_change can do this — its Interaction never changed)",
    );
    assert_ne!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*new.button.color),
        "the held-hover button must NOT be stuck on the NEW resting base after the reload",
    );

    Ok(())
}

#[test]
fn held_pressed_button_repaints_to_new_pressed_on_theme_change()
-> Result<(), ron::error::SpannedError> {
    let old = theme(
        [0.08, 0.08, 0.10, 1.0],
        [0.20, 0.20, 0.24, 1.0],
        [0.04, 0.04, 0.06, 1.0],
    )?;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(ScenePlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);
    app.insert_resource(old.clone());

    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(&mut commands, &old, ButtonLabel::new("Fight"), ())
    };
    app.world_mut().flush();
    app.update();
    app.update();

    set_interaction(&mut app, button, Interaction::Pressed);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*old.button.pressed),
        "precondition: the held press must show the OLD pressed_bg before the reload",
    );

    let new = theme(
        [0.50, 0.10, 0.30, 1.0],
        [0.90, 0.70, 0.10, 1.0],
        [0.10, 0.40, 0.80, 1.0],
    )?;
    app.insert_resource(new.clone());
    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*new.button.pressed),
        "a held-press button must repaint to the NEW pressed_bg the same frame the theme \
         changes (only repaint_theme_change can do this — its Interaction never changed)",
    );

    Ok(())
}

#[test]
fn theme_change_repaint_leaves_disabled_and_active_buttons() -> Result<(), ron::error::SpannedError>
{
    let old = theme(
        [0.08, 0.08, 0.10, 1.0],
        [0.20, 0.20, 0.24, 1.0],
        [0.04, 0.04, 0.06, 1.0],
    )?;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(ScenePlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);
    app.insert_resource(old.clone());

    let (disabled, active) = {
        let mut commands = app.world_mut().commands();
        let disabled = spawn_button(
            &mut commands,
            &old,
            ButtonLabel::new("Locked"),
            DisabledButton,
        );
        let active = spawn_button(&mut commands, &old, ButtonLabel::new("Aim"), ActiveButton);
        (disabled, active)
    };
    app.world_mut().flush();
    app.update();
    app.update();

    set_interaction(&mut app, disabled, Interaction::Hovered);
    set_interaction(&mut app, active, Interaction::Hovered);
    app.update();

    let new = theme(
        [0.50, 0.10, 0.30, 1.0],
        [0.90, 0.70, 0.10, 1.0],
        [0.10, 0.40, 0.80, 1.0],
    )?;
    app.insert_resource(new.clone());
    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(disabled).map(|c| c.0),
        Some(*new.button.disabled),
        "a disabled button must keep the NEW disabled fill across the reload, never a \
         hover/resting fill",
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(active).map(|c| c.0),
        Some(*new.button.active),
        "an active toggle must keep the NEW active fill across the reload, never a \
         hover/resting fill",
    );

    Ok(())
}
