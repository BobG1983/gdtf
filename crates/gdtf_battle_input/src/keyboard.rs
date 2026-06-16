//! The keyboard press surface (GTW-225 / GTW-48 S8): the systems that read the
//! data-driven [`Keybinds`] and PUSH the matching [`ActIntent`] onto the shared seam.
//!
//! These systems own the no-act keys THIS slice handles — select-clear, level-up,
//! level-down — reading the bound [`KeyCode`] off the resident [`Keybinds`] resource
//! ([`BoundKey::key_code`](crate::keybinds::BoundKey::key_code), NO hardcoded literal)
//! and `push`ing the intent so the ONE [`dispatch_act_intents`](crate::dispatch_act_intents)
//! drain acts on it. The act-bearing keys (stance / aim / facing / fire-mode) are
//! DECLARED in the same [`Keybinds`] table and read by 222b's keyboard surface, which
//! pushes the act-bearing [`ActIntent`] variants onto this SAME seam.
//!
//! Gated `run_if(resource_exists::<Keybinds>)` by the plugin (in addition to the
//! battle gate): the table is asset-loaded, so a `MinimalPlugins` headless app with no
//! `AssetServer` never resolves it and these systems simply do not run (no panic) —
//! a test that wants them inserts [`Keybinds`] directly.

use bevy::prelude::*;

use crate::{ActIntent, Keybinds, PendingActIntent};

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
