use bevy::{
    MinimalPlugins,
    input::InputPlugin,
    prelude::*,
    scene::ScenePlugin,
    ui::{BackgroundColor, Interaction},
};
use cobalt_test_utils::unwatched_asset_plugin;

use super::support::theme;
use crate::{
    UiPlugin,
    widgets::core::{ActiveButton, ButtonLabel, spawn_button},
};

#[test]
fn deactivated_button_repaints_to_resting_same_frame() -> Result<(), ron::error::SpannedError> {
    let panel = [0.08, 0.08, 0.10, 1.0];
    let hover = [0.20, 0.20, 0.24, 1.0];
    let press = [0.04, 0.04, 0.06, 1.0];
    let theme_res = theme(panel, hover, press)?;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(unwatched_asset_plugin())
        .add_plugins(ScenePlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);
    app.insert_resource(theme_res.clone());

    let (a, b) = {
        let mut commands = app.world_mut().commands();
        let a = spawn_button(
            &mut commands,
            &theme_res,
            ButtonLabel::new("Single"),
            ActiveButton,
        );
        let b = spawn_button(&mut commands, &theme_res, ButtonLabel::new("Burst"), ());
        (a, b)
    };
    app.world_mut().flush();

    app.update();
    app.update();

    assert_ne!(
        *theme_res.button.active, *theme_res.button.color,
        "test precondition: the active and resting fills must differ",
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).map(|c| c.0),
        Some(*theme_res.button.active),
        "precondition: A starts painted the active fill",
    );

    app.world_mut().entity_mut(a).remove::<ActiveButton>();
    app.world_mut().entity_mut(b).insert(ActiveButton);
    assert_eq!(
        app.world().get::<Interaction>(a).copied(),
        Some(Interaction::None),
        "precondition: A's Interaction stays None across the toggle switch",
    );

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(a).map(|c| c.0),
        Some(*theme_res.button.color),
        "A must repaint to the RESTING fill the frame it loses ActiveButton \
         (only repaint_deactivated_buttons can do this — its Interaction never changed)",
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(b).map(|c| c.0),
        Some(*theme_res.button.active),
        "B must show the ACTIVE fill the frame it becomes the new selection",
    );

    Ok(())
}
