use bevy::{
    prelude::*,
    ui::{BackgroundColor, Interaction},
};

use super::{paint_active_buttons, paint_disabled_buttons};
use crate::{
    theme::GdtfTheme,
    themed::apply_theme,
    widgets::core::{
        ActiveButton, ButtonLabel, DisabledButton, spawn_button,
        test_support::{scene_app, theme},
    },
};

#[test]
fn disabled_button_is_painted_from_theme() -> Result<(), ron::error::SpannedError> {
    use crate::themed::Themed;

    let mut app = scene_app();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;
    app.insert_resource(theme_res.clone());
    app.add_systems(
        Update,
        (apply_theme, paint_disabled_buttons)
            .chain()
            .run_if(resource_exists::<GdtfTheme>),
    );

    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(
            &mut commands,
            &theme_res,
            ButtonLabel::new("Locked"),
            DisabledButton,
        )
    };
    app.world_mut().flush();

    app.update();

    let world = app.world();
    let expected = *theme_res.button.disabled;
    assert_eq!(
        world.get::<BackgroundColor>(button).map(|c| c.0),
        Some(expected),
        "disabled button must be the button sub-theme's explicit disabled fill",
    );
    assert!(
        world.get::<Themed>(button).is_some(),
        "disabled button must remain Themed so apply_theme still reaches it",
    );

    Ok(())
}

#[test]
fn active_button_is_painted_from_theme() -> Result<(), ron::error::SpannedError> {
    use crate::themed::Themed;

    let mut app = scene_app();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;
    app.insert_resource(theme_res.clone());
    app.add_systems(
        Update,
        (apply_theme, paint_active_buttons)
            .chain()
            .run_if(resource_exists::<GdtfTheme>),
    );

    let (active, plain) = {
        let mut commands = app.world_mut().commands();
        let active = spawn_button(
            &mut commands,
            &theme_res,
            ButtonLabel::new("Aim"),
            ActiveButton,
        );
        let plain = spawn_button(&mut commands, &theme_res, ButtonLabel::new("Aim"), ());
        (active, plain)
    };
    app.world_mut().flush();

    app.update();

    let world = app.world();
    assert_ne!(
        *theme_res.button.active, *theme_res.button.color,
        "test precondition: the active and resting fills must differ",
    );
    assert_eq!(
        world.get::<BackgroundColor>(active).map(|c| c.0),
        Some(*theme_res.button.active),
        "an ActiveButton must be the button sub-theme's explicit active fill",
    );
    assert_eq!(
        world.get::<BackgroundColor>(plain).map(|c| c.0),
        Some(*theme_res.button.color),
        "a button WITHOUT ActiveButton must keep the resting base color",
    );
    assert!(
        world.get::<Themed>(active).is_some(),
        "an active button must remain Themed so apply_theme still reaches it",
    );

    Ok(())
}

#[test]
fn active_button_stays_active_when_pressed() -> Result<(), ron::error::SpannedError> {
    use crate::{themed::UiSystems, widgets::interaction::theme_interaction};

    let mut app = scene_app();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;
    app.insert_resource(theme_res.clone());
    app.add_systems(
        Update,
        (
            apply_theme.in_set(UiSystems::ApplyTheme),
            theme_interaction.after(UiSystems::ApplyTheme),
            paint_active_buttons.after(theme_interaction),
        )
            .run_if(resource_exists::<GdtfTheme>),
    );

    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(
            &mut commands,
            &theme_res,
            ButtonLabel::new("Aim"),
            ActiveButton,
        )
    };
    app.world_mut().flush();

    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
    app.update();

    assert_ne!(
        *theme_res.button.active, *theme_res.button.pressed,
        "test precondition: the active and pressed fills must differ",
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*theme_res.button.active),
        "an ActiveButton must keep its ACTIVE fill when pressed (active is STICKY — the \
         interaction path EXCLUDES ActiveButton)",
    );

    Ok(())
}

#[test]
fn disabled_beats_active() -> Result<(), ron::error::SpannedError> {
    let mut app = scene_app();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;
    app.insert_resource(theme_res.clone());
    app.add_systems(
        Update,
        (apply_theme, paint_disabled_buttons, paint_active_buttons)
            .chain()
            .run_if(resource_exists::<GdtfTheme>),
    );

    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(
            &mut commands,
            &theme_res,
            ButtonLabel::new("Aim"),
            (DisabledButton, ActiveButton),
        )
    };
    app.world_mut().flush();

    app.update();

    assert_ne!(
        *theme_res.button.disabled, *theme_res.button.active,
        "test precondition: the disabled and active fills must differ",
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*theme_res.button.disabled),
        "a disabled+active button must show the DISABLED fill (active paint skips \
         disabled)",
    );

    Ok(())
}

#[test]
fn deselected_toggle_drops_active_fill_without_hover() -> Result<(), ron::error::SpannedError> {
    use bevy::input::InputPlugin;

    use crate::UiPlugin;

    let mut app = scene_app();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;
    app.add_plugins(InputPlugin).add_plugins(UiPlugin);
    app.insert_resource(theme_res.clone());

    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(
            &mut commands,
            &theme_res,
            ButtonLabel::new("Single"),
            ActiveButton,
        )
    };
    app.world_mut().flush();

    app.update();
    app.update();
    assert_ne!(
        *theme_res.button.active, *theme_res.button.color,
        "test precondition: the active and resting fills must differ",
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*theme_res.button.active),
        "precondition: the toggle starts painted the active fill",
    );

    app.world_mut().entity_mut(button).remove::<ActiveButton>();
    assert_eq!(
        app.world().get::<Interaction>(button).copied(),
        Some(Interaction::None),
        "precondition: the de-selected toggle's Interaction stays None",
    );

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*theme_res.button.color),
        "a de-selected toggle must drop the active fill back to RESTING the same \
         frame, with no hover (repaint_deactivated_buttons)",
    );

    Ok(())
}
