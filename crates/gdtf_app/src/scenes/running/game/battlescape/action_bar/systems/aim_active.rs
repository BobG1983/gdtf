//! [`sync_aim_button_active`] — drive the Aim button's active (toggled-on) look from
//! the selected ganger's [`Aiming`] state (GTW-253).
//!
//! Play-test bug #6: the Aim action-bar button fires
//! [`ActIntent::AimToggle`](gdtf_battle_input::ActIntent::AimToggle) but shows no
//! on/off state, so the player can't tell whether the selected ganger is aiming. This
//! system supplies the missing VISUAL feedback — it does NOT change how the toggle
//! works (the existing
//! [`ActIntent::AimToggle`](gdtf_battle_input::ActIntent::AimToggle) →
//! [`SetAimingRequested`](gdtf_battle_sim::acts::SetAimingRequested) → sim-flips-[`Aiming`]
//! chain is untouched). It mirrors the sim's [`Aiming`] onto the `gdtf_ui`
//! [`ActiveButton`](gdtf_ui::ActiveButton) visual primitive on the
//! [`AimToggleButton`](super::super::components::AimToggleButton) entity.
//!
//! ## The sim → button mapping
//!
//! Each battle frame it reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter)
//! → the selected entity → its [`Aiming`] component:
//!
//! - selected ganger is aiming → INSERT [`ActiveButton`](gdtf_ui::ActiveButton) on the
//!   Aim button (the `gdtf_ui` `paint_active_buttons` pass then shows the theme's active
//!   fill);
//! - selected ganger is NOT aiming → REMOVE it (button shows OFF);
//! - no selection, or the selected entity has no [`Aiming`] component → REMOVE it
//!   (button shows OFF).
//!
//! The marker is purely visual (`gdtf_ui` treats [`ActiveButton`](gdtf_ui::ActiveButton)
//! as a paint-only signal, NOT an interaction filter), so the Aim button stays fully
//! clickable to toggle OFF.
//!
//! ## Gating + ordering (`bevy-traps.md` #1 / #3 / #7)
//!
//! Registered `run_if(resource_exists::<BattleInProgress>)` by the action-bar plugin —
//! the SAME live-battle witness the other action-bar systems gate on — so it is inert
//! when no battle is live (the bar is only spawned in `BattleRunning` anyway). It uses
//! [`Commands`] for the insert/remove (NOT `&mut World`). The look reflects the CURRENT
//! [`Aiming`]; because the sim applies the toggle in its own dispatch, the button look
//! may lag the press by at most one frame, which is acceptable for a visual indicator.

use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::Aiming;
use gdtf_ui::{ActiveButton, ButtonLabel, spawn_button, theme::GdtfTheme};

use crate::scenes::running::game::battlescape::action_bar::components::AimToggleButton;

/// Spawns the **Aim toggle** button ([`AimToggleButton`]), sized to FILL its parent cell, and
/// returns its [`Entity`] so a caller can parent it under the host layout cell (GTW-298).
///
/// The reusable Aim-toggle constructor (the `spawn_mode_panel` / `spawn_stance_panel`
/// precedent): GTW-298 relocated the Aim toggle into the weapon-cluster's Aim Panel, spawned by
/// the weapon-panel module through this shared constructor — NOT inside the action bar. The
/// press → intent routing (`action_bar_button_intents`) and the active-mark sync
/// ([`sync_aim_button_active`]) are UNCHANGED: they query the [`AimToggleButton`] marker
/// parent-agnostically, so the toggle works wherever it is parented. Takes `&mut Commands` +
/// the live theme.
pub(in crate::scenes::running::game::battlescape) fn spawn_aim_button(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    let button = spawn_button(commands, theme, ButtonLabel::new("Aim"), AimToggleButton);
    // FILL the Aim Panel box (GTW-298). Overwriting the auto-sized `box_node` is safe —
    // `apply_theme` re-applies the theme-owned border / radius / padding every run, preserving
    // these layout fields.
    commands.entity(button).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    });
    button
}

/// Syncs the [`ActiveButton`](gdtf_ui::ActiveButton) marker on the
/// [`AimToggleButton`](super::super::components::AimToggleButton) entity to the
/// selected ganger's [`Aiming`] state (GTW-253).
///
/// Reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter); if it holds an
/// entity whose [`Aiming`] component is `true`, the Aim button gets
/// [`ActiveButton`](gdtf_ui::ActiveButton) inserted (the `gdtf_ui`
/// `paint_active_buttons` pass then paints the theme's active fill). Otherwise — not
/// aiming, no selection, or the selected entity carries no [`Aiming`] — the marker is
/// removed, so the button shows OFF. Insert/remove are idempotent (a re-insert of the
/// unit marker, or a remove when already absent, is harmless), so this can run every
/// frame.
///
/// This is VISUAL-ONLY: the marker does not affect interaction (`gdtf_ui` paints
/// `ActiveButton` but never filters it out of the interaction path), so the Aim button
/// stays clickable to toggle the aim back off.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert/remove, a
/// `Res<`[`SelectedShooter`](gdtf_battle_input::SelectedShooter)`>` read, a read-only
/// `Query<&`[`Aiming`]`>` (the selected ganger's aim flag), and a
/// `Query<`[`Entity`]`, With<`[`AimToggleButton`](super::super::components::AimToggleButton)`>>`
/// to find the one Aim button — no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn sync_aim_button_active(
    mut commands: Commands,
    selected: Res<SelectedShooter>,
    aiming: Query<&Aiming>,
    aim_buttons: Query<Entity, With<AimToggleButton>>,
) {
    // The selected ganger is aiming iff there is a selection whose `Aiming` is true.
    // No selection, or a selected entity without an `Aiming` component, reads as OFF.
    let is_aiming = (**selected)
        .and_then(|entity| aiming.get(entity).ok())
        .is_some_and(|aim| **aim);

    for button in &aim_buttons {
        let mut entity = commands.entity(button);
        if is_aiming {
            entity.insert(ActiveButton);
        } else {
            entity.remove::<ActiveButton>();
        }
    }
}
