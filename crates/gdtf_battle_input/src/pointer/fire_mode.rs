//! The selected **fire mode** (GTW-227 / GTW-48 S8 222b): the [`SelectedFireMode`]
//! resource the FIRE surface feeds into [`FireRequested`](gdtf_battle_sim::acts::FireRequested).
//!
//! The selected mode is the per-mode numbers ([`FireModeSpec`]) the next shot fires
//! with. On selecting an armed ganger it is set to that weapon's
//! [`FireMode::single`](gdtf_battle_sim::FireMode::single) (the `Single`-kind mode).
//!
//! GTW-254 REPLACED the blind cycle (the removed `next_fire_mode` helper + the
//! fire-mode-cycle key/button) with the `gdtf_app` popup picker, which SETS
//! [`SelectedFireMode`] directly to a mode read back off the selected weapon's
//! [`FireMode`](gdtf_battle_sim::FireMode) selector. This module now owns only the
//! resource + its default-on-select sync ([`sync_fire_mode_on_select`]).

use bevy::prelude::*;
use gdtf_battle_sim::{FireMode, FireModeSpec, WieldedBy, Wields};

use crate::SelectedShooter;

/// The currently SELECTED fire mode — the per-mode numbers the next shot uses.
///
/// A named newtype over [`FireModeSpec`] (no-bare-types: the selected mode is a
/// domain value; [`Resource`] is the framework carve-out) that [`Deref`]s to its
/// inner spec so the FIRE surface reads `*selected_fire_mode` straight into a
/// [`FireRequested`](gdtf_battle_sim::acts::FireRequested). `init_resource`-d by
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) to a structural
/// single-shot default ([`Default`]), then RESET by the selection path
/// ([`sync_fire_mode_on_select`]) to the picked weapon's
/// [`FireMode::single`](gdtf_battle_sim::FireMode::single), and SET directly by the
/// `gdtf_app` popup picker (GTW-254) to a mode read back off the selected weapon.
///
/// `Copy` again — it holds a [`FireModeSpec`], which regained `Copy` once the
/// `String` mode name was dropped (GTW-260). The `Deref`-into-[`gdtf_battle_sim::acts::FireRequested`] read
/// stays unchanged.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq)]
pub struct SelectedFireMode(FireModeSpec);

impl SelectedFireMode {
    /// Build a selected fire mode holding `spec`.
    #[must_use]
    pub const fn new(spec: FireModeSpec) -> Self {
        Self(spec)
    }
}

impl Default for SelectedFireMode {
    /// The structural single-shot default — `Single` kind, `cone_mult = 1`,
    /// `tu_percent = 0`, `shots = 1`. A spawn-time placeholder, NOT a balance value:
    /// the selection path resets it to the picked weapon's authored
    /// [`FireMode::single`] the moment an armed ganger is selected, so this default is
    /// only ever live before any selection.
    fn default() -> Self {
        Self(FireModeSpec::new(
            gdtf_battle_sim::ModeKind::Single,
            gdtf_battle_sim::ModeConeMult::new(1.0),
            gdtf_battle_sim::ModeTuPercent::new(0.0),
            gdtf_battle_sim::ModeShots::new(1),
        ))
    }
}

/// On a CHANGE of [`SelectedShooter`] to an armed ganger — OR when a wielded weapon's
/// [`FireMode`] first becomes queryable (the deferred-spawn race below) — RESET
/// [`SelectedFireMode`] to that weapon's [`FireMode::single`] (GTW-227 / 222b AC1;
/// GTW-376).
///
/// Runs `.after(left_click_act)` so it observes the SAME update's selection. When
/// the [`SelectedShooter`] resource changed this update (a fresh selection), it
/// resolves the selected ganger's WIELDED WEAPON ENTITY (`ganger → `[`Wields`]` → the
/// weapon entity`, GTW-323 slice 3 — the [`FireMode`] selector lives on the weapon
/// entity now, not the ganger) and sets [`SelectedFireMode`] to its
/// [`FireMode::single`] — the base mode present on every variant, the documented
/// default on selecting an armed ganger. A selection of an UNARMED entity (no
/// [`Wields`], no weapon, or no [`FireMode`] on the weapon) leaves the mode untouched
/// (fail-closed via the relationship + query lookup); clearing the selection likewise
/// leaves it (nothing to read). Only writes on a real change of the resulting mode
/// (change-detection hygiene).
///
/// GTW-376 — it ALSO re-resolves when ANY weapon's [`FireMode`] was just spawned
/// ([`Added<FireMode>`](Added)). The wielded weapon entity is spawned by
/// `setup_battle` via the framework's deferred `queue_spawn_related_scenes::<Wields>`
/// (GTW-323 slice 2), which applies a FRAME LATER than the ganger spawn — so the
/// GTW-255 battle-start [`auto_select_first_player_ganger`](crate::auto_select_first_player_ganger) flips
/// [`SelectedShooter`] before the weapon's [`FireMode`] is queryable, leaving this
/// system's `selected.is_changed()` branch to fail-close and the mode stuck at the
/// `tu_percent: 0` [`Default`] (the fire-target highlight then reads "0 TU"). Mirroring
/// the action-bar `rebuild_mode_segments` / `sync_mode_tu_cost_lines`
/// `Added<ModeControl>` re-trigger, the `Added<FireMode>` signal re-resolves the
/// CURRENT selection's mode the frame the weapon arrives, so the resolved
/// [`SelectedFireMode`] (the same value the shot charges via `mode_tu_cost`) becomes
/// non-zero without a click. The write stays change-guarded, so the extra trigger is
/// idempotent on a quiet update.
///
/// Param-only (`bevy-traps.md` #7): a `Res<SelectedShooter>` read, a read-only
/// `Query<&Wields>`, a `Query<&FireMode, With<WieldedBy>>` weapon-entity query, an
/// [`Added<FireMode>`](Added) detector, and the `ResMut<SelectedFireMode>` write.
pub fn sync_fire_mode_on_select(
    selected: Res<SelectedShooter>,
    wields: Query<&Wields>,
    weapons: Query<&FireMode, With<WieldedBy>>,
    weapon_just_armed: Query<(), Added<FireMode>>,
    mut fire_mode: ResMut<SelectedFireMode>,
) {
    // React on a real selection change OR when a weapon's FireMode was JUST spawned (the
    // GTW-376 deferred-weapon-spawn race — the battle-start auto-select flips the selection
    // a frame BEFORE the wielded weapon's FireMode is queryable, so the selection change has
    // passed by the time the weapon exists; re-resolving on its arrival recovers the cost).
    let weapon_arrived = weapon_just_armed.iter().next().is_some();
    if !selected.is_changed() && !weapon_arrived {
        return;
    }
    let Some(shooter) = **selected else {
        // Selection cleared — leave the mode (it rides until the next armed select).
        return;
    };
    // The FireMode selector lives on the wielded WEAPON entity (GTW-323 slice 3):
    // resolve `ganger → Wields → the weapon entity → FireMode`. An unarmed shooter
    // (no Wields, no weapon, or no FireMode on the weapon) leaves the mode untouched.
    let Some(weapon) = wields
        .get(shooter)
        .ok()
        .and_then(Wields::weapon)
        .and_then(|weapon| weapons.get(weapon).ok())
    else {
        return;
    };
    let next = SelectedFireMode::new(weapon.single());
    if *fire_mode != next {
        *fire_mode = next;
    }
}
