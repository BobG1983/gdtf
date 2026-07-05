//! Test for the GTW-280 deactivation repaint: losing `ActiveButton` repaints
//! the button to its resting fill the same frame, with no `Interaction` change.

use bevy::{
    MinimalPlugins,
    asset::AssetPlugin,
    input::InputPlugin,
    prelude::*,
    scene::ScenePlugin,
    ui::{BackgroundColor, Interaction},
};

use super::support::theme;
use crate::{
    UiPlugin,
    widgets::core::{ActiveButton, ButtonLabel, spawn_button},
};

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
