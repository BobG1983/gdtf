//! Tests for the widget spawn helpers and the disabled / active paint passes.

use bevy::{
    prelude::*,
    text::{FontSize, FontSource, TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, Node, widget::Button},
};

use super::{
    ActiveButton, ButtonLabel, DisabledButton, paint_active_buttons, paint_disabled_buttons,
    spawn_button, spawn_panel,
};
use crate::{
    theme::{GdtfTheme, GdtfThemeSpec},
    themed::{ThemeRole, Themed, apply_theme},
};

/// Builds a [`GdtfTheme`] through the real resolution path (deserialize the
/// nested spec, then [`GdtfThemeSpec::resolve`]) with a defaulted-font
/// resolver. The button sub-theme's fill is caller-chosen so the paint tests
/// can pin it. The theme newtypes have private fields, so this respects
/// encapsulation while exercising the genuine runtime resolution. Returns the
/// `ron` error so a malformed literal surfaces via `?`.
fn theme(
    button_color: [f32; 4],
    disabled: [f32; 4],
) -> Result<GdtfTheme, ron::error::SpannedError> {
    let [pr, pg, pb, pa] = button_color;
    let [dr, dg, db, da] = disabled;
    let ron = format!(
        "(\
         default_font: \"fonts/test.ttf\", \
         background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
         panel: ( color: (0.16, 0.16, 0.18, 0.55), border_color: (0.20, 0.20, 0.24, 1.0), \
                  border_width: 2.0, corner_radius: 5.0, \
                  margin: (left: 12.0, right: 12.0, top: 6.0, bottom: 6.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, {pa}), disabled: ({dr}, {dg}, {db}, {da}), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: (0.80, 0.16, 0.19, 0.96), pressed: (0.10, 0.10, 0.12, 0.96), \
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

/// `spawn_button` builds a button carrying the interaction plumbing
/// (`Button` + `Interaction`), the button-box visuals (`BackgroundColor` +
/// `BorderColor`), and the `Themed(Button)` marker.
///
/// Pin-discriminating: dropping any of those components, or the Themed
/// marker, fails an assert.
#[test]
fn spawned_button_has_interaction_visuals_and_themed_marker() -> Result<(), ron::error::SpannedError>
{
    let mut app = App::new();
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

/// `spawn_panel` builds a `Themed(Panel)` node with background + border.
#[test]
fn spawned_panel_is_themed_with_visuals() -> Result<(), ron::error::SpannedError> {
    let mut app = App::new();
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

/// The text child of a `spawn_button` carries the button sub-theme font handle,
/// size, and text color, and is itself `Themed(ButtonText)`.
#[test]
fn spawned_button_text_child_is_themed_button_text_from_theme()
-> Result<(), ron::error::SpannedError> {
    let mut app = App::new();
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

/// A `DisabledButton`'s background is the button sub-theme's explicit
/// `disabled` fill after `paint_disabled_buttons` runs, and it stays `Themed`.
#[test]
fn disabled_button_is_painted_from_theme() -> Result<(), ron::error::SpannedError> {
    let mut app = App::new();
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

/// AC1 — an `ActiveButton` ends up with the button sub-theme's explicit `active`
/// fill after the paint pass, while the SAME button WITHOUT `ActiveButton` keeps
/// the resting `color` base.
///
/// Pin-discriminating (compares against the theme's own value, never a hardcoded
/// magnitude — the active color is tunable data): if `paint_active_buttons`
/// failed to apply the active color, the active assert would still see the base
/// `color` and fail; if it painted the plain button too, the plain assert would
/// fail. The two fills are distinct in the test theme.
#[test]
fn active_button_is_painted_from_theme() -> Result<(), ron::error::SpannedError> {
    let mut app = App::new();
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
    // The active fill and the resting base are distinct, so the asserts discriminate.
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

/// AC2 (GTW-266 — active is STICKY) — an `ActiveButton` keeps its ACTIVE fill even when
/// `Interaction::Pressed` (or `Hovered`): `theme_interaction` now EXCLUDES `ActiveButton`
/// (`Without<ActiveButton>`), so a toggled-on button never takes a hover/press swap, and
/// `paint_active_buttons` (ordered after) is the only writer of its color. This is the
/// fixed UX behind the user's "Aim button flickers when active + hovered" complaint and is
/// required by the Mode/Stance toggle panels.
///
/// Pin-discriminating: the active fill and the pressed fill are distinct in the test theme,
/// so if `theme_interaction` ever stopped excluding `ActiveButton`, a pressed active button
/// would swap to the PRESSED fill and this assert would fail. (This replaces the
/// pre-GTW-266 assert that an active button SHOULD hover/press-swap — that behavior was the
/// flicker bug.)
#[test]
fn active_button_stays_active_when_pressed() -> Result<(), ron::error::SpannedError> {
    use bevy::ui::Interaction;

    use crate::{interaction::theme_interaction, themed::UiSystems, widgets::paint_active_buttons};

    let mut app = App::new();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;
    app.insert_resource(theme_res.clone());
    // The real production band: apply_theme (its set), then theme_interaction, then
    // paint_active_buttons (ordered .after(theme_interaction) — the GTW-266 last writer).
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

    // Press the active button — a hover/press swap a real pointer would drive.
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
    app.update();

    // Test precondition: the active and pressed fills differ, so the assert discriminates.
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

/// AC3 — disabled beats active: a button that is BOTH `DisabledButton` and
/// `ActiveButton` shows the DISABLED fill, because `paint_active_buttons` skips
/// disabled buttons (`Without<DisabledButton>`).
///
/// Pin-discriminating: if `paint_active_buttons` dropped its
/// `Without<DisabledButton>` filter, the active fill would win and this assert
/// (expecting the disabled fill) would fail.
#[test]
fn disabled_beats_active() -> Result<(), ron::error::SpannedError> {
    let mut app = App::new();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;
    app.insert_resource(theme_res.clone());
    // The real production band: apply_theme, then both paint passes.
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

/// GTW-280, AC5 (mirror) — a just-de-selected toggle is repainted to its resting
/// fill the SAME frame it loses `ActiveButton`, driven by the REAL [`UiPlugin`]
/// path (`MinimalPlugins` + `InputPlugin` + `UiPlugin`) so the new
/// `repaint_deactivated_buttons` system runs in its production ordering.
///
/// Mirrors the interaction-side test from the widgets/paint angle: this is the
/// generic `gdtf_ui` defect the Mode and Stance toggle panels hit — switching the
/// active toggle must repaint the previously-selected one without waiting for a
/// hover. The button's `Interaction` is left `Interaction::None` throughout, so
/// `theme_interaction` (`Changed<Interaction>`) cannot account for the repaint —
/// only `repaint_deactivated_buttons` can.
///
/// Pin-discriminating: remove `repaint_deactivated_buttons` from [`UiPlugin`] and
/// the de-selected button stays stuck on the active fill, failing the assert.
#[test]
fn deselected_toggle_drops_active_fill_without_hover() -> Result<(), ron::error::SpannedError> {
    use bevy::{MinimalPlugins, input::InputPlugin};

    use crate::UiPlugin;

    let mut app = App::new();
    let theme_res = theme([0.12, 0.12, 0.15, 1.0], [0.08, 0.08, 0.10, 0.55])?;
    app.add_plugins(MinimalPlugins)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);
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

    // Settle the spawn over two updates: the first lets apply_theme paint the base
    // (change-driven on the NEW Themed entity), the second lets `Added<Themed>`
    // clear so apply_theme will NOT re-run on the DECIDING frame below — otherwise
    // it would re-paint the button to its resting base for free and the test would
    // stop discriminating the fix. paint_active_buttons keeps the active fill across
    // both settles.
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

    // De-select WITHOUT touching Interaction (it stays None).
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
