//! The keyboard press surface (GTW-225 / GTW-48 S8 + GTW-227 / 222b): the systems
//! that read the data-driven [`Keybinds`] and PUSH the matching [`ActIntent`] onto the
//! shared seam.
//!
//! 222a's no-act keys — select-clear, level-up, level-down ([`select_clear_key`] /
//! [`level_keys`]) — read the bound [`KeyCode`] off the resident [`Keybinds`] resource
//! ([`BoundKey::key_code`](crate::keybinds::BoundKey::key_code), NO hardcoded literal)
//! and `push` the intent so the ONE
//! [`dispatch_act_intents`](crate::dispatch_act_intents) drain acts on it.
//!
//! 222b (GTW-227) adds the act-bearing keys — stance-cycle / aim-toggle / facing-cycle
//! ([`posture_keys`]) — reading the bound [`KeyCode`] off the SAME [`Keybinds`] table
//! and pushing the act-bearing [`ActIntent`] variants ([`ActIntent::StanceCycle`] /
//! [`ActIntent::AimToggle`] / [`ActIntent::FacingCycle`]) onto the SAME seam the 222c
//! buttons write. These act keys only push WHEN a [`SelectedShooter`] is set — with no
//! selection they write NO intent (AC6). (The blind fire-mode-cycle key was REMOVED in
//! GTW-254 — fire-mode selection is now the `gdtf_app` popup picker.)
//!
//! Gated `run_if(resource_exists::<Keybinds>)` by the plugin (in addition to the
//! battle gate): the table is asset-loaded, so a `MinimalPlugins` headless app with no
//! `AssetServer` never resolves it and these systems simply do not run (no panic) —
//! a test that wants them inserts [`Keybinds`] directly.

use bevy::prelude::*;

use crate::{ActIntent, Keybinds, PendingActIntent, SelectedShooter};

/// Reads the level-up / level-down keys and PUSHES the matching level [`ActIntent`].
///
/// On a `just_pressed` of the [`Keybinds::level_up`] / [`Keybinds::level_down`] key,
/// pushes [`ActIntent::LevelUp`] / [`ActIntent::LevelDown`] onto the
/// [`PendingActIntent`] queue — the seam the
/// [`dispatch_act_intents`](crate::dispatch_act_intents) drain clamps into
/// `ActiveLevel`. No `KeyCode` literal: the bound code is read off the loaded
/// [`Keybinds`] resource. Param-only (`bevy-traps.md` #7).
pub fn level_keys(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<Keybinds>,
    mut pending: ResMut<PendingActIntent>,
) {
    if keys.just_pressed(binds.level_up()) {
        pending.push(ActIntent::LevelUp);
    }
    if keys.just_pressed(binds.level_down()) {
        pending.push(ActIntent::LevelDown);
    }
}

/// Reads the select-clear key and PUSHES [`ActIntent::SelectionClear`].
///
/// On a `just_pressed` of the [`Keybinds::select_clear`] key, pushes
/// [`ActIntent::SelectionClear`] onto the [`PendingActIntent`] queue — the seam the
/// drain uses to clear [`SelectedShooter`](crate::SelectedShooter). No `KeyCode`
/// literal: the bound code is read off the loaded [`Keybinds`] resource. Param-only
/// (`bevy-traps.md` #7).
pub fn select_clear_key(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<Keybinds>,
    mut pending: ResMut<PendingActIntent>,
) {
    if keys.just_pressed(binds.select_clear()) {
        pending.push(ActIntent::SelectionClear);
    }
}

/// Reads the stance-cycle / aim-toggle / facing-cycle keys and PUSHES the matching
/// act-bearing posture [`ActIntent`] for the [`SelectedShooter`] (GTW-227 / 222b).
///
/// On a `just_pressed` of the [`Keybinds::stance_cycle`] / [`Keybinds::aim_toggle`] /
/// [`Keybinds::facing_cycle`] key — and ONLY while a [`SelectedShooter`] is set —
/// pushes [`ActIntent::StanceCycle`] / [`ActIntent::AimToggle`] /
/// [`ActIntent::FacingCycle`] onto the [`PendingActIntent`] queue. The drain reads the
/// selected actor's CURRENT posture, steps the authored [`crate::cycle`] order, and
/// emits the matching `gdtf_battle_sim::acts::Set*Requested` — the SAME seam the 222c
/// buttons write. With no selection it writes NO intent (AC6). No `KeyCode` literal:
/// the bound codes are read off the loaded [`Keybinds`] resource. Param-only
/// (`bevy-traps.md` #7).
pub fn posture_keys(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<Keybinds>,
    selected: Res<SelectedShooter>,
    mut pending: ResMut<PendingActIntent>,
) {
    // With no selection these act keys are inert — they write no intent (AC6).
    if selected.is_none() {
        return;
    }
    if keys.just_pressed(binds.stance_cycle()) {
        pending.push(ActIntent::StanceCycle);
    }
    if keys.just_pressed(binds.aim_toggle()) {
        pending.push(ActIntent::AimToggle);
    }
    if keys.just_pressed(binds.facing_cycle()) {
        pending.push(ActIntent::FacingCycle);
    }
}
