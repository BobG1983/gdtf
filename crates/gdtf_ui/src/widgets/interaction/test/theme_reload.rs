//! Tests for the GTW-147 theme-change repaint (mirrors `repaint.rs`'s second
//! writer) plus the hot-reload live-theme-read contract of the same reload
//! path: a held hover/press repaints from the NEW theme the frame it changes,
//! and the reload repaint leaves disabled / active buttons to their own paints.

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

/// GTW-147 — a button held `Hovered` across a `GdtfTheme` hot-reload is repainted to the
/// NEW theme's `hover_bg` the SAME frame the theme changes, WITHOUT its `Interaction` ever
/// changing.
///
/// Drives the REAL [`UiPlugin`] path (`MinimalPlugins` + `InputPlugin` + `UiPlugin`): the
/// plugin registers `apply_theme` (which on a theme change repaints the button to its base,
/// IGNORING `Interaction`), `theme_interaction` (which only fires on `Changed<Interaction>`),
/// and the GTW-147 `repaint_theme_change` in their production ordering. A themed button is
/// spawned and settled, then hovered so it shows the OLD `hover_bg`. The theme is then
/// overwritten with a DIFFERENT `hover_bg` WITHOUT touching the button's `Interaction`, and
/// the app updates ONCE.
///
/// Pin-discriminating: because the button's `Interaction` never changes after the hover,
/// `theme_interaction` (`Changed<Interaction>`) never fires for it on the reload frame; only
/// `apply_theme` (→ NEW base) and `repaint_theme_change` (→ NEW `hover_bg`) run. Remove
/// `repaint_theme_change` from [`UiPlugin`] and the button is stuck on the NEW base instead
/// of the NEW `hover_bg`, and this assert fails.
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

    // Settle the spawn so `Added<Themed>` clears: without this, the reload frame's
    // `apply_theme` would run regardless and the test still discriminates, but two
    // settling updates keep the steady-state precondition unambiguous.
    app.update();
    app.update();

    // Hover the button (the swap a real pointer drives) and let `theme_interaction`
    // land the OLD `hover_bg`.
    set_interaction(&mut app, button, Interaction::Hovered);
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(*old.button.hover),
        "precondition: the held hover must show the OLD hover_bg before the reload",
    );

    // Hot-reload: overwrite `GdtfTheme` with a deliberately different `hover_bg` (and a
    // different resting base, so a clobber to the base is detectable). Crucially, the
    // button's `Interaction` is NOT touched — it stays `Interaction::Hovered` — so only
    // `repaint_theme_change` can repaint it to the new hover this frame.
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

    // One update is all the fix gets.
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

/// GTW-147 (pressed analog) — a button held `Pressed` across a `GdtfTheme` hot-reload is
/// repainted to the NEW theme's `pressed_bg` the SAME frame, without its `Interaction`
/// changing.
///
/// Same shape and discrimination as
/// [`held_hover_button_repaints_to_new_hover_on_theme_change`]: only `repaint_theme_change`
/// repaints a `Pressed` button whose `Interaction` is unchanged on the reload frame.
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

/// GTW-147 — `repaint_theme_change` leaves a `DisabledButton` and an `ActiveButton` on the
/// fill their own paint owns across a theme reload: a disabled button keeps the disabled
/// fill (`paint_disabled_buttons`), an active toggle keeps the active fill
/// (`paint_active_buttons`). The reload-repaint's write set is DISJOINT from those special
/// paints.
///
/// Pin-discriminating: dropping `Without<DisabledButton>`/`Without<ActiveButton>` from
/// `repaint_theme_change`'s filter would let it write the resting/hover fill over the
/// disabled / active fill (it runs in the same band, and ordering between the two writers is
/// unconstrained), and these asserts would fail.
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

    // Hover both (the disabled is skipped by interaction anyway; the active is sticky).
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
