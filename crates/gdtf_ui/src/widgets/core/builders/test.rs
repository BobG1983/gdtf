use bevy::{
    prelude::*,
    text::{FontSize, FontSource, TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, Node, widget::Button},
};

use super::{spawn_button, spawn_panel};
use crate::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
    widgets::core::{
        ButtonLabel,
        test_support::{scene_app, theme},
    },
};

#[test]
fn spawned_button_has_interaction_visuals_and_themed_marker() -> Result<(), ron::error::SpannedError>
{
    let mut app = scene_app();
    app.insert_resource(theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?);

    let theme_res = app.world().resource::<GdtfTheme>().clone();
    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(&mut commands, &theme_res, ButtonLabel::new("Fight"), ())
    };
    app.world_mut().flush();

    let world = app.world();
    assert!(world.get::<Button>(button).is_some(), "must have Button");
    assert!(
        world.get::<Interaction>(button).is_some(),
        "Button requires Interaction",
    );
    assert!(
        world.get::<BackgroundColor>(button).is_some(),
        "must have BackgroundColor",
    );
    assert!(
        world.get::<UiBorderColor>(button).is_some(),
        "must have BorderColor",
    );
    assert_eq!(
        world.get::<Themed>(button).map(|t| **t),
        Some(ThemeRole::Button),
        "must carry Themed(Button)",
    );

    Ok(())
}

#[test]
fn spawned_panel_is_themed_with_visuals() -> Result<(), ron::error::SpannedError> {
    let mut app = scene_app();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;

    let panel = {
        let mut commands = app.world_mut().commands();
        spawn_panel(&mut commands, &theme_res)
    };
    app.world_mut().flush();

    let world = app.world();
    assert!(world.get::<Node>(panel).is_some(), "must have Node");
    assert!(
        world.get::<BackgroundColor>(panel).is_some(),
        "must have BackgroundColor",
    );
    assert!(
        world.get::<UiBorderColor>(panel).is_some(),
        "must have BorderColor",
    );
    assert_eq!(
        world.get::<Themed>(panel).map(|t| **t),
        Some(ThemeRole::Panel),
        "must carry Themed(Panel)",
    );

    Ok(())
}

#[test]
fn spawned_button_text_child_is_themed_button_text_from_theme()
-> Result<(), ron::error::SpannedError> {
    let mut app = scene_app();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;

    let button = {
        let mut commands = app.world_mut().commands();
        spawn_button(&mut commands, &theme_res, ButtonLabel::new("Fight"), ())
    };
    app.world_mut().flush();

    let world = app.world();
    let children = world
        .get::<Children>(button)
        .map(|c| c.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    assert_eq!(children.len(), 1, "button must have exactly one text child");
    let child = children[0];

    assert_eq!(
        world.get::<Themed>(child).map(|t| **t),
        Some(ThemeRole::ButtonText),
        "text child must be Themed(ButtonText)",
    );
    assert_eq!(
        world.get::<TextFont>(child).map(|f| f.font.clone()),
        Some(FontSource::from(Handle::<Font>::default())),
        "text child must carry the button font handle",
    );
    assert!(
        world
            .get::<TextFont>(child)
            .is_some_and(|f| f.font_size == FontSize::Px(18.0)),
        "text child must carry the button font size",
    );
    assert_eq!(
        world.get::<UiTextColor>(child).map(|c| c.0),
        Some(Color::srgb(0.84, 0.80, 0.73)),
        "text child must carry the button text color",
    );

    Ok(())
}
