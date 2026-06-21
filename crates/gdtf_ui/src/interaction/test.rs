//! Tests for the theme-derived interaction layer and the hover→focus bridge.

use bevy::{
    MinimalPlugins,
    asset::AssetPlugin,
    input::InputPlugin,
    input_focus::InputFocus,
    prelude::*,
    scene::ScenePlugin,
    ui::{BackgroundColor, Interaction, widget::Button},
};

use super::theme_interaction;
use crate::{
    UiPlugin,
    theme::{GdtfTheme, GdtfThemeSpec},
    themed::{UiSystems, apply_theme},
    widgets::{
        ActiveButton, ButtonLabel, DisabledButton, Orientation, SwitchColors, SwitchState,
        spawn_button, spawn_switch,
    },
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
                  border_width: 2.0, corner_radius: 5.0, \
                  margin: (left: 12.0, right: 12.0, top: 6.0, bottom: 6.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, {pa}), disabled: (0.08, 0.08, 0.10, 0.55), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: ({hr}, {hg}, {hb}, {ha}), pressed: ({sr}, {sg}, {sb}, {sa}), \
                   text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0, \
                   border_color: (0.20, 0.20, 0.24, 1.0), \
                   border_width: 2.0, corner_radius: 5.0, \
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
///
/// Includes `MinimalPlugins` + `AssetPlugin` + `ScenePlugin` (GTW-322) so the
/// widget `bsn!` builders' `Commands::spawn_scene` resolves on flush instead of
/// panicking on the missing scene/asset resources.
fn app_with_interaction() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
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
/// [`sync_hover_to_focus`] would let the hover land focus and this assert
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

/// GTW-280, AC5 — a button that LOSES `ActiveButton` is repainted to its resting
/// fill the SAME frame, without its `Interaction` ever changing.
///
/// Drives the REAL [`UiPlugin`] path (`MinimalPlugins` + `InputPlugin` +
/// `UiPlugin`): the plugin registers `apply_theme`, `paint_active_buttons`,
/// `theme_interaction`, and the GTW-280 `repaint_deactivated_buttons` in their
/// production ordering. Two themed buttons A and B are spawned; A starts active
/// and is painted the active fill. Then `ActiveButton` is moved from A to B (the
/// real toggle-switch: the previous selection becomes the new one) WITHOUT
/// touching A's `Interaction` (it stays `Interaction::None`), and the app updates
/// ONCE.
///
/// Pin-discriminating: because A's `Interaction` never changes, `theme_interaction`
/// (`Changed<Interaction>`) never fires for it and `paint_active_buttons` no longer
/// matches it — ONLY `repaint_deactivated_buttons` can repaint A. So if that system
/// is removed from [`UiPlugin`], A stays stuck on the ACTIVE fill and the A-assert
/// fails. The B-assert confirms the new selection is painted active the same frame.
#[test]
fn deactivated_button_repaints_to_resting_same_frame() -> Result<(), ron::error::SpannedError> {
    let panel = [0.08, 0.08, 0.10, 1.0];
    let hover = [0.20, 0.20, 0.24, 1.0];
    let press = [0.04, 0.04, 0.06, 1.0];
    let theme_res = theme(panel, hover, press)?;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
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

    // Settle the spawn: the first update lets apply_theme paint the base look (it
    // is change-driven on a NEW Themed entity); a second update lets `Added<Themed>`
    // clear so apply_theme will NOT re-run on the DECIDING frame below. Without
    // this, apply_theme would re-paint A to its resting base for free and the test
    // would no longer discriminate the fix (it would pass even with the new system
    // removed). paint_active_buttons keeps A's active fill across both settling
    // updates.
    app.update();
    app.update();

    // Test precondition: active and resting fills differ, so the asserts discriminate.
    assert_ne!(
        *theme_res.button.active, *theme_res.button.color,
        "test precondition: the active and resting fills must differ",
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(a).map(|c| c.0),
        Some(*theme_res.button.active),
        "precondition: A starts painted the active fill",
    );

    // The real toggle-switch: move ActiveButton from A to B. Crucially, A's
    // Interaction is NOT touched — it stays Interaction::None — so only the new
    // deactivation-repaint system can repaint A.
    app.world_mut().entity_mut(a).remove::<ActiveButton>();
    app.world_mut().entity_mut(b).insert(ActiveButton);
    assert_eq!(
        app.world().get::<Interaction>(a).copied(),
        Some(Interaction::None),
        "precondition: A's Interaction stays None across the toggle switch",
    );

    // One update is all the fix gets.
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
