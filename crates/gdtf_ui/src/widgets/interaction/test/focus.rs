use bevy::{
    MinimalPlugins,
    asset::AssetPlugin,
    input::InputPlugin,
    input_focus::InputFocus,
    prelude::*,
    scene::ScenePlugin,
    ui::{Interaction, widget::Button},
};

use crate::{UiPlugin, widgets::core::DisabledButton};

#[test]
fn hover_moves_input_focus_to_button() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(ScenePlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);

    let button = app.world_mut().spawn((Button, Interaction::Hovered)).id();

    app.update();

    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(button),
        "hovering an enabled button must move InputFocus onto it",
    );
}

#[test]
fn hover_does_not_focus_disabled_button() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(ScenePlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);

    app.world_mut()
        .spawn((Button, Interaction::Hovered, DisabledButton));

    app.update();

    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        None,
        "a hovered disabled button must NOT move InputFocus",
    );
}
