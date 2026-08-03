use bevy::{
    prelude::*,
    ui::{BackgroundColor, Interaction, widget::Button},
};

use super::{
    super::theme_interaction,
    support::{app_with_interaction, set_interaction, theme},
};
use crate::{
    theme::GdtfTheme,
    widgets::core::{
        ActiveButton, ButtonLabel, DisabledButton, Orientation, SwitchColors, SwitchState,
        spawn_button, spawn_switch,
    },
};

#[test]
fn hover_and_press_swap_to_theme_state_colors() -> Result<(), ron::error::SpannedError> {
    let panel = [0.08, 0.08, 0.10, 1.0];
    let hover = [0.20, 0.20, 0.24, 1.0];
    let press = [0.04, 0.04, 0.06, 1.0];
    let theme_res = theme(panel, hover, press)?;

    let mut app = app_with_interaction();
    app.insert_resource(theme_res.clone());

    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(&mut commands, &theme_res, ButtonLabel::new("Fight"), ())
    };
    app.world_mut().flush();

    set_interaction(&mut app, button, Interaction::Hovered);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*theme_res.button.hover),
        "hovered button must show HoverBg",
    );

    set_interaction(&mut app, button, Interaction::Pressed);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*theme_res.button.pressed),
        "pressed button must show PressBg",
    );

    set_interaction(&mut app, button, Interaction::None);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*theme_res.button.color),
        "released button must return to the resting PanelBg",
    );

    Ok(())
}

#[test]
fn disabled_button_is_skipped_by_interaction() -> Result<(), ron::error::SpannedError> {
    let panel = [0.08, 0.08, 0.10, 1.0];
    let hover = [0.20, 0.20, 0.24, 1.0];
    let press = [0.04, 0.04, 0.06, 1.0];
    let theme_res = theme(panel, hover, press)?;

    let mut app = app_with_interaction();
    app.insert_resource(theme_res.clone());

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
    let base = app.world().get::<BackgroundColor>(button).map(|c| c.0);

    set_interaction(&mut app, button, Interaction::Hovered);
    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        base,
        "disabled button's background must be unchanged by a Hovered interaction",
    );
    assert_ne!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*theme_res.button.hover),
        "disabled button must never show HoverBg",
    );

    Ok(())
}

#[test]
fn active_button_is_skipped_by_interaction() -> Result<(), ron::error::SpannedError> {
    let panel = [0.08, 0.08, 0.10, 1.0];
    let hover = [0.20, 0.20, 0.24, 1.0];
    let press = [0.04, 0.04, 0.06, 1.0];
    let theme_res = theme(panel, hover, press)?;

    let mut app = app_with_interaction();
    app.insert_resource(theme_res.clone());

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
    app.update();
    let base = app.world().get::<BackgroundColor>(button).map(|c| c.0);

    set_interaction(&mut app, button, Interaction::Hovered);
    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        base,
        "an active button's background must be unchanged by a Hovered interaction \
         (theme_interaction skips ActiveButton)",
    );
    assert_ne!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*theme_res.button.hover),
        "an active button must never show HoverBg (active is sticky)",
    );

    Ok(())
}

#[test]
fn absent_theme_does_not_panic() {
    let mut app = App::new();
    app.add_systems(Update, theme_interaction);
    app.world_mut()
        .spawn((Button, Interaction::Hovered, BackgroundColor(Color::WHITE)));

    app.update();

    assert!(
        app.world().get_resource::<GdtfTheme>().is_none(),
        "test precondition: GdtfTheme must be absent for this guard check",
    );
}

#[test]
fn switch_track_is_skipped_by_interaction() -> Result<(), ron::error::SpannedError> {
    let panel = [0.16, 0.16, 0.18, 1.0];
    let hover = [0.20, 0.20, 0.24, 1.0];
    let press = [0.04, 0.04, 0.06, 1.0];
    let theme_res = theme(panel, hover, press)?;

    let off_track = Color::srgb(0.58, 0.58, 0.59);
    let colors = SwitchColors {
        off:  off_track,
        on:   *theme_res.button.active,
        knob: *theme_res.button.text_color,
    };

    let mut app = app_with_interaction();
    app.insert_resource(theme_res.clone());

    let switch = {
        let mut commands = app.world_mut().commands();
        spawn_switch(
            &mut commands,
            SwitchState::Off,
            colors,
            Orientation::Horizontal,
            (),
        )
    };
    app.world_mut().flush();

    assert_ne!(
        off_track, *theme_res.button.color,
        "test precondition: the OFF-track color must differ from the resting button fill",
    );

    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(switch).map(|c| c.0),
        Some(off_track),
        "the switch track must KEEP its OFF color across the spawn frame \
         (theme_interaction skips Switch)",
    );

    set_interaction(&mut app, switch, Interaction::Hovered);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(switch).map(|c| c.0),
        Some(off_track),
        "a hovered switch track must STILL keep its OFF color (theme_interaction skips Switch)",
    );
    assert_ne!(
        app.world().get::<BackgroundColor>(switch).map(|c| c.0),
        Some(*theme_res.button.color),
        "the switch track must never be repainted to the resting button fill",
    );

    Ok(())
}
