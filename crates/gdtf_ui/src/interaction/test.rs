//! Tests for the theme-derived interaction layer and the hover→focus bridge.

use bevy::{
    MinimalPlugins,
    input::InputPlugin,
    input_focus::InputFocus,
    prelude::*,
    ui::{BackgroundColor, Interaction, widget::Button},
};

use super::theme_interaction;
use crate::{
    UiPlugin,
    theme::{GdtfTheme, GdtfThemeSpec},
    themed::{UiSystems, apply_theme},
    widgets::{ButtonLabel, DisabledButton, spawn_button},
};

/// Builds a [`GdtfTheme`] with caller-chosen button resting / hover / pressed
/// colors through the real resolution path (deserialize the nested spec, then
/// [`GdtfThemeSpec::resolve`]) with a defaulted-font resolver. Returns the
/// `ron` error so a malformed literal surfaces via `?` rather than a denied
/// `unwrap`/`panic`.
fn theme(
    button_color: [f32; 4],
    hover: [f32; 4],
    pressed: [f32; 4],
) -> Result<GdtfTheme, ron::error::SpannedError> {
    let [pr, pg, pb, pa] = button_color;
    let [hr, hg, hb, ha] = hover;
    let [sr, sg, sb, sa] = pressed;
    let ron = format!(
        "(\
         default_font: \"fonts/test.ttf\", \
         background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
         panel: ( color: (0.16, 0.16, 0.18, 0.55), border_color: (0.20, 0.20, 0.24, 1.0), \
                  border_width_px: 2.0, corner_radius_px: 5.0, \
                  margin: (left: 12.0, right: 12.0, top: 6.0, bottom: 6.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, {pa}), disabled: (0.08, 0.08, 0.10, 0.55), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: ({hr}, {hg}, {hb}, {ha}), pressed: ({sr}, {sg}, {sb}, {sa}), \
                   text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0, \
                   border_color: (0.20, 0.20, 0.24, 1.0), \
                   border_width_px: 2.0, corner_radius_px: 5.0, \
                   margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
         title: ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 36.0 ), \
         text:  ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0 ))",
    );
    let spec: GdtfThemeSpec = ron::from_str(&ron)?;
    Ok(spec.resolve(|_| Handle::<Font>::default()))
}

/// Builds a minimal app with the real production schedule: `apply_theme`
/// (in its named set) before `theme_interaction`, both under the live run
/// condition — mirroring [`UiPlugin`](crate::UiPlugin)'s wiring.
fn app_with_interaction() -> App {
    let mut app = App::new();
    app.add_systems(
        Update,
        (
            apply_theme.in_set(UiSystems::ApplyTheme),
            theme_interaction.after(UiSystems::ApplyTheme),
        )
            .run_if(resource_exists::<GdtfTheme>),
    );
    app
}

/// Sets a button's [`Interaction`] in the world (the swap a real pointer
/// would otherwise drive), so the test can exercise the state transitions
/// headlessly.
fn set_interaction(app: &mut App, button: Entity, state: Interaction) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = state;
    }
}

/// `theme_interaction` swaps a hovered button to `HoverBg` and a pressed
/// button to `PressBg`, both sourced from the theme — AC#4.
///
/// Pin-discriminating: a wrong state→color mapping, or hardcoded literals,
/// fail an assert.
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

/// A `DisabledButton` is skipped by `theme_interaction`: simulating Hovered
/// leaves its background at the `apply_theme` base, never `HoverBg` — AC#5.
///
/// Pin-discriminating: dropping `Without<DisabledButton>` would let the
/// hover swap fire and this assert would see `HoverBg`.
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
    // Establish the base look first.
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

/// Hot-reload contract: after spawn + `apply_theme`, replacing `GdtfTheme`
/// with a NEW palette repaints the resting button to the NEW base, and a
/// subsequently-hovered button shows the NEW `HoverBg` — proving the
/// interaction layer reads the live theme each run, never a spawn snapshot
/// (AC#4b, test strategy #4).
///
/// Pin-discriminating: if `theme_interaction` cached colors at spawn, the
/// post-reload hover would still show the OLD `HoverBg` and the assert fails.
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

    // Hot-reload: a deliberately different palette.
    let new = theme(
        [0.50, 0.10, 0.30, 1.0],
        [0.90, 0.70, 0.10, 1.0],
        [0.10, 0.40, 0.80, 1.0],
    )?;
    app.insert_resource(new.clone());

    // Re-run: apply_theme repaints the resting base from the NEW theme.
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*new.button.color),
        "resting button must reflect the NEW base after hot-reload",
    );

    // A subsequently-hovered button must show the NEW HoverBg.
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

/// With no `GdtfTheme`, an `app.update()` does not panic: the
/// `Option<Res<GdtfTheme>>` guard keeps `theme_interaction` inert
/// (bevy-traps rule 1).
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

/// Hovering an enabled button moves [`InputFocus`] onto it, so mouse hover
/// and keyboard / gamepad navigation share one focus cursor — AC#2.
///
/// Drives the full [`UiPlugin`] wiring headlessly (`MinimalPlugins` +
/// `UiPlugin`): `UiPlugin` installs the focus-nav layer (initializing
/// [`InputFocus`]) and registers [`sync_hover_to_focus`] in the
/// `.after(UiSystems::ApplyTheme)` band. Setting [`Interaction::Hovered`] is
/// the swap `ui_focus_system` would otherwise drive from a real cursor (that
/// path needs a window and is local-only in-engine evidence, GTW-123).
///
/// Pin-discriminating: dropping the `focus.set` call, or the
/// `== Interaction::Hovered` guard, leaves focus empty and this assert fails.
///
/// `InputPlugin` is added because `UiPlugin`'s focus-nav layer pulls in
/// `InputDispatchPlugin`, whose dispatch systems require the input message
/// buffers `InputPlugin` registers — `MinimalPlugins` alone panics
/// "Message not initialized" (bevy-traps rule 1 / the headless-input
/// prerequisite).
#[test]
fn hover_moves_input_focus_to_button() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
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
/// [`sync_hover_to_focus`] would let the hover land focus and this assert
/// (focus stays empty) would fail.
#[test]
fn hover_does_not_focus_disabled_button() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
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
