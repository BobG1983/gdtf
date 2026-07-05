//! Tests for the `theme_interaction` hover/press swaps and its exclusion
//! filters (mirrors the source `theme.rs`): the state → theme-color mapping,
//! the disabled / active / switch-track skips, and the absent-theme guard.

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

/// GTW-266 — `theme_interaction` SKIPS an `ActiveButton`: simulating Hovered (the swap a
/// real pointer would drive) leaves its background at the `apply_theme` base, never
/// `HoverBg`. Active is STICKY — the interaction feedback never overrides a toggled-on
/// button (its color comes solely from `paint_active_buttons`), so the Aim/Mode/Stance
/// toggle does not flicker hover-vs-active.
///
/// Pin-discriminating: dropping the `Without<ActiveButton>` filter from `theme_interaction`
/// would let the hover swap fire and this assert would see `HoverBg`.
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
    // Establish the base look first (apply_theme paints the resting base).
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

/// GTW-277 (track-visibility fix) — `theme_interaction` SKIPS a `Switch` track: its track
/// `BackgroundColor` is OWNED by the switch driver / caller, so the resting `button.color`
/// fill never clobbers the switch's distinct off-track color on the frame its `Interaction`
/// is added (spawn) or changes (hover).
///
/// A switch track IS a `Button` (a click anywhere flips it), so without `Without<Switch>` in
/// `theme_interaction`'s filter the resting fill would overwrite the off-track color the frame
/// the switch's `Interaction` was added — leaving the track painted the near-panel
/// `button.color` and reading as a bare knob with no visible pill (the reported defect).
///
/// Pin-discriminating: dropping `Without<Switch>` lets the spawn-frame `Changed<Interaction>`
/// repaint the track to `button.color`, and BOTH asserts (track keeps its OFF color; track is
/// NOT `button.color`) fail. The OFF color here is deliberately distinct from `button.color`.
#[test]
fn switch_track_is_skipped_by_interaction() -> Result<(), ron::error::SpannedError> {
    let panel = [0.16, 0.16, 0.18, 1.0];
    let hover = [0.20, 0.20, 0.24, 1.0];
    let press = [0.04, 0.04, 0.06, 1.0];
    let theme_res = theme(panel, hover, press)?;

    // A switch OFF-track color clearly distinct from the resting button fill (so a clobber is
    // detectable) and from the active/hover/pressed fills.
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

    // Test precondition: the OFF track color and the resting button fill differ, so a clobber
    // would be visible.
    assert_ne!(
        off_track, *theme_res.button.color,
        "test precondition: the OFF-track color must differ from the resting button fill",
    );

    // The spawn frame adds `Interaction` (Changed) — the frame `theme_interaction` would
    // clobber the track if the switch were not excluded.
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(switch).map(|c| c.0),
        Some(off_track),
        "the switch track must KEEP its OFF color across the spawn frame \
         (theme_interaction skips Switch)",
    );

    // A subsequent hover is another `Changed<Interaction>` — still must not clobber.
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
