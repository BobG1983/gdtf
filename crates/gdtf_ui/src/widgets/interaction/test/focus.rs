//! Tests for the hover-to-focus bridge (mirrors the source `focus.rs`): a
//! hovered enabled button takes [`InputFocus`]; a disabled one never does.

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

/// Hovering an enabled button moves [`InputFocus`] onto it, so mouse hover
/// and keyboard / gamepad navigation share one focus cursor — AC#2.
///
/// Drives the full [`UiPlugin`] wiring headlessly (`MinimalPlugins` +
/// `UiPlugin`): `UiPlugin` installs the focus-nav layer (initializing
/// [`InputFocus`]) and registers `sync_hover_to_focus` in the
/// `.after(UiSystems::ApplyTheme)` band. Setting [`Interaction::Hovered`] is
/// the swap `ui_focus_system` would otherwise drive from a real cursor (that
/// path needs a window and is local-only in-engine evidence, GTW-123).
///
/// Pin-discriminating: dropping the `focus.set` call, or the
/// `== Interaction::Hovered` guard, leaves focus empty and this assert fails.
///
/// `InputPlugin` is added so `UiPlugin`'s focus-nav bridge has the
/// `ButtonInput<KeyCode>` resource / keyboard message buffers it reads — under
/// bare `MinimalPlugins` those are absent. (As of Bevy 0.19 the
/// `InputDispatchPlugin` that owns `InputFocus` ships in `DefaultPlugins`, not in
/// `UiPlugin`; `FocusNavPlugin` `init_resource`s `InputFocus` itself so this
/// `MinimalPlugins` harness still has it — bevy-traps rule 1.)
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

/// Hovering a [`DisabledButton`] does **not** move [`InputFocus`]: a disabled
/// button never steals focus — AC#2.
///
/// Pin-discriminating: dropping the `Without<DisabledButton>` filter on
/// `sync_hover_to_focus` would let the hover land focus and this assert
/// (focus stays empty) would fail.
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
