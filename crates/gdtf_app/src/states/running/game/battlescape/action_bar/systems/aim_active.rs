//! The Aim control — a `gdtf_ui` [`Switch`] — its press → intent wiring and its
//! sim-driven on/off sync (GTW-253 / GTW-277).
//!
//! GTW-277 migrated the Aim control from an ad-hoc [`spawn_button`](gdtf_ui::spawn_button)
//! toggle to the generic `gdtf_ui` [`Switch`] widget. The control is the AIM toggle in the
//! mockup: a rounded track whose color tells the on/off state with a sliding knob. It
//! keeps the [`AimToggleButton`] identity marker so the relocated control is found
//! parent-agnostically wherever the weapon-panel module parents it (GTW-298).
//!
//! ## Press → intent (the 222a seam, byte-equal to the key surface)
//!
//! Clicking the switch makes `gdtf_ui`'s [`drive_switches`](gdtf_ui::drive_switches) flip
//! it and emit a [`ToggleFlipped`](gdtf_ui::ToggleFlipped) message carrying the switch's
//! [`Entity`]. [`aim_switch_flip_intent`] reads that message, confirms the flipped switch
//! carries the [`AimToggleButton`] marker, and pushes the SAME
//! [`ActIntent::AimToggle`](gdtf_battle_input::ActIntent::AimToggle) the aim KEY pushes —
//! so the button + key surfaces stay parallel over the ONE
//! [`PendingActIntent`](gdtf_battle_input::PendingActIntent) drain (ADR-0001), and the
//! emitted [`SetAimingRequested`](gdtf_battle_sim::acts::SetAimingRequested) is
//! byte-for-byte equal between the two surfaces.
//!
//! ## Sim → switch sync (no feedback loop)
//!
//! [`sync_aim_switch_state`] mirrors the selected ganger's
//! [`Aiming`](gdtf_battle_sim::Aiming) onto the switch's [`SwitchState`](gdtf_ui::SwitchState):
//! aiming → [`SwitchState::On`], not-aiming / no-selection → [`SwitchState::Off`]. Writing
//! [`SwitchState`] alone does NOT repaint the track (`drive_switches` only repaints on a
//! user click), so this system ALSO re-derives the track color + re-justifies the knob to
//! match — the sim-driven set is a pure visual sync that NEVER re-emits a
//! [`ToggleFlipped`] (no feedback loop: it writes the state component + the look directly,
//! never an `Interaction`).

use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::Aiming;
use gdtf_ui::{
    Orientation, Switch, SwitchColors, SwitchOrientation, SwitchState, ToggleFlipped, spawn_switch,
    theme::GdtfTheme,
};

use crate::states::running::game::battlescape::action_bar::components::AimToggleButton;

/// Spawns the **Aim toggle** as a `gdtf_ui` horizontal [`Switch`] ([`AimToggleButton`]),
/// sized to FILL its parent cell, and returns its [`Entity`] so a caller can parent it
/// under the host layout cell (GTW-277 / GTW-298).
///
/// The reusable Aim-switch constructor (the `spawn_mode_panel` / `spawn_stance_panel`
/// precedent): GTW-298 relocated the Aim toggle into the weapon-cluster's Aim Panel,
/// spawned by the weapon-panel module through this shared constructor — NOT inside the
/// action bar. GTW-277 swapped the underlying widget to a [`Switch`]: the track off/on
/// colors come from the theme (off = the resting button fill, on = the active fill — so
/// the on look matches the rest of the HUD's "engaged" color), the knob is the theme's
/// button text color. The press → intent routing ([`aim_switch_flip_intent`]) and the
/// on/off sync ([`sync_aim_switch_state`]) find this switch by its [`AimToggleButton`]
/// marker parent-agnostically, so it works wherever it is parented. Takes `&mut Commands`
/// + the live theme.
pub(in crate::states::running::game::battlescape) fn spawn_aim_button(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    spawn_switch(
        commands,
        SwitchState::Off,
        aim_switch_colors(theme),
        Orientation::Horizontal,
        AimToggleButton,
    )
    // GTW-298: the `Switch` is a self-contained widget sized by its own track geometry (a
    // fixed `Vw`), UNLIKE the old fill-the-box button. The Aim Panel cell (the weapon-panel
    // module's `spawn_right_column`) CENTRES it, so the switch keeps its intrinsic size and
    // reads as a compact toggle inside the framed cell — no `Node` override needed here.
}

/// The track off/on + knob colors a relocated Aim [`Switch`] paints with, derived from the
/// live theme so the switch reads as part of the HUD (GTW-277).
///
/// The track must read as a visible PILL against the Aim Panel's `Themed(Panel)` fill
/// (`theme.panel.color`, a near-black `~(0.09, 0.09, 0.10)` over the backdrop). The theme's
/// resting button fill (`~(0.16, 0.16, 0.18)`) is too close to that panel to read as a
/// track, so the OFF state instead uses a distinctly LIGHTER, opaque neutral grey —
/// [`off_track_color`] lightens the resting button fill — giving the off pill clear contrast
/// with the panel it sits on. ON = the theme's active / toggled-on fill (the same `active`
/// green a `paint_active_buttons` button uses, matching the mockup's green on-pill), so off
/// vs on is obvious; the knob is the theme's button text color (a constant bone-white pip).
/// The theme's color newtypes [`Deref`] to [`Color`].
fn aim_switch_colors(theme: &GdtfTheme) -> SwitchColors {
    SwitchColors {
        off:  off_track_color(*theme.button.color),
        on:   *theme.button.active,
        knob: *theme.button.text_color,
    }
}

/// Lightens the theme's resting button fill into the OFF-state track color: a clearly
/// brighter, fully-opaque neutral grey that reads as a visible pill against the near-black
/// Aim Panel fill (GTW pill-visibility fix).
///
/// Lerps each sRGB channel halfway toward white and forces full opacity, so the off track is
/// distinctly lighter than both the panel (`~0.09`) and the resting button (`~0.16`) without
/// hardcoding a color the theme doesn't own — it stays a function of the live theme.
fn off_track_color(button: Color) -> Color {
    /// Fraction lightened toward white (halfway = a clearly brighter neutral grey).
    const LIGHTEN: f32 = 0.5;
    let srgba = button.to_srgba();
    Color::srgb(
        (1.0 - srgba.red).mul_add(LIGHTEN, srgba.red),
        (1.0 - srgba.green).mul_add(LIGHTEN, srgba.green),
        (1.0 - srgba.blue).mul_add(LIGHTEN, srgba.blue),
    )
}

/// Pushes [`ActIntent::AimToggle`] when the Aim [`Switch`] is flipped by the user
/// (GTW-277 / GTW-253).
///
/// Reads [`ToggleFlipped`](gdtf_ui::ToggleFlipped) messages (emitted by
/// [`drive_switches`](gdtf_ui::drive_switches) on a real click), and for each whose
/// flipped switch carries the [`AimToggleButton`] marker, [`push`](PendingActIntent::push)es
/// the SAME [`ActIntent::AimToggle`] the aim KEY pushes onto the shared 222a seam — so the
/// switch + key surfaces stay parallel over the ONE drain (the byte-equal parity the
/// `aim_button_toggles_and_matches_direct_intent` test pins). The buttons write NO
/// `*Requested` directly; the ONE [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents)
/// drain interprets the pushed intent against the `SelectedShooter`, so a flip with no
/// selection is a no-op in the drain (AC6).
///
/// Param-only (`bevy-traps.md` #7): a [`MessageReader<ToggleFlipped>`](MessageReader)
/// (bevy-traps rule 4), the [`ResMut<PendingActIntent>`](ResMut) write, and a read-only
/// `Query<(), With<AimToggleButton>>` — no `&mut World`.
pub(in crate::states::running::game::battlescape) fn aim_switch_flip_intent(
    mut flipped: MessageReader<ToggleFlipped>,
    mut pending: ResMut<PendingActIntent>,
    aim_switches: Query<(), With<AimToggleButton>>,
) {
    for event in flipped.read() {
        if aim_switches.get(event.switch).is_ok() {
            pending.push(ActIntent::AimToggle);
        }
    }
}

/// Read-write [`Query`] data for the Aim [`Switch`] during a sim sync: its
/// [`SwitchState`], its [`SwitchColors`], its [`SwitchOrientation`], its
/// [`BackgroundColor`](bevy::ui::BackgroundColor) (the track fill), and its
/// [`Node`] (the track's knob justify).
///
/// Named to keep [`sync_aim_switch_state`]'s signature legible (clippy `type_complexity`).
type AimSwitchData = (
    &'static mut SwitchState,
    &'static SwitchColors,
    &'static SwitchOrientation,
    &'static mut BackgroundColor,
    &'static mut Node,
);

/// Query FILTER selecting the Aim [`Switch`] root (the [`AimToggleButton`]-marked
/// [`Switch`]).
///
/// Aliased so [`sync_aim_switch_state`]'s `Query<AimSwitchData, …>` type stays legible
/// (clippy `type_complexity`).
type AimSwitchFilter = (With<AimToggleButton>, With<Switch>);

/// Syncs the Aim [`Switch`]'s on/off state + look to the selected ganger's
/// [`Aiming`](gdtf_battle_sim::Aiming) (GTW-277 / GTW-253).
///
/// Reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter); if it holds an
/// entity whose [`Aiming`] is `true`, the switch is set [`SwitchState::On`]; otherwise —
/// not aiming, no selection, or the selected entity carries no [`Aiming`] — it is set
/// [`SwitchState::Off`]. Because `gdtf_ui`'s [`drive_switches`](gdtf_ui::drive_switches)
/// only repaints the track on a USER click, this system ALSO re-derives the track
/// [`BackgroundColor`](bevy::ui::BackgroundColor) and re-justifies the knob to match the
/// new state, so a sim-driven flip is visible (the contract's "if writing `SwitchState`
/// alone doesn't repaint, add the repaint path"). Writes only on a real change
/// (change-detection hygiene), and it NEVER re-emits a
/// [`ToggleFlipped`](gdtf_ui::ToggleFlipped) — it writes the state + look directly, never
/// an `Interaction`, so there is no feedback loop (sim → switch only).
///
/// Param-only (`bevy-traps.md` #7): a `Res<SelectedShooter>` read, a read-only
/// `Query<&Aiming>`, and an [`AimSwitchData`] write query over the switch root (the knob is
/// the switch root's child; only the root's own `Node` justify slides it, so no separate knob
/// query is needed) — no `&mut World`.
pub(in crate::states::running::game::battlescape) fn sync_aim_switch_state(
    selected: Res<SelectedShooter>,
    aiming: Query<&Aiming>,
    mut switches: Query<AimSwitchData, AimSwitchFilter>,
) {
    // The selected ganger is aiming iff there is a selection whose `Aiming` is true.
    // No selection, or a selected entity without an `Aiming` component, reads as OFF.
    let is_aiming = (**selected)
        .and_then(|entity| aiming.get(entity).ok())
        .is_some_and(|aim| **aim);
    let want = if is_aiming {
        SwitchState::On
    } else {
        SwitchState::Off
    };

    for (mut state, colors, orientation, mut background, mut node) in &mut switches {
        if *state != want {
            *state = want;
            // Re-derive the look the way `drive_switches` does on a click, since this is a
            // sim-driven set (no `Interaction` change to trigger the driver). No re-emit.
            background.0 = colors.track(want);
            node.flex_direction = orientation.flex_direction();
            node.justify_content = knob_justify(want);
        }
    }
}

/// Which end of the track the knob sits at for a [`SwitchState`] — mirrors the
/// `gdtf_ui` switch's own internal mapping (OFF = start, ON = end), applied here on the
/// sim-driven sync path so a state set without a click still slides the knob.
const fn knob_justify(state: SwitchState) -> JustifyContent {
    match state {
        SwitchState::Off => JustifyContent::FlexStart,
        SwitchState::On => JustifyContent::FlexEnd,
    }
}
