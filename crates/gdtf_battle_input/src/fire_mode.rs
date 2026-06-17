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
use gdtf_battle_sim::{FireMode, FireModeSpec};

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
/// `String` mode name was dropped (GTW-260). The `Deref`-into-[`FireRequested`] read
/// stays unchanged.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq)]
pub struct SelectedFireMode(pub FireModeSpec);

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

/// On a CHANGE of [`SelectedShooter`] to an armed ganger, RESET [`SelectedFireMode`]
/// to that weapon's [`FireMode::single`] (GTW-227 / 222b AC1).
///
/// Runs `.after(left_click_act)` so it observes the SAME update's selection. When
/// the [`SelectedShooter`] resource changed this update (a fresh selection), it looks
/// up the selected entity's [`FireMode`] selector and sets [`SelectedFireMode`] to its
/// [`FireMode::single`] — the base mode present on every variant, the documented
/// default on selecting an armed ganger. A selection of an UNARMED entity (no
/// [`FireMode`] component) leaves the mode untouched (fail-closed via the query
/// lookup); clearing the selection likewise leaves it (nothing to read). Only writes
/// on a real change of the resulting mode (change-detection hygiene). Param-only
/// (`bevy-traps.md` #7): `Res<SelectedShooter>` + a read-only `&FireMode` query, the
/// `ResMut<SelectedFireMode>` write.
pub fn sync_fire_mode_on_select(
    selected: Res<SelectedShooter>,
    weapons: Query<&FireMode>,
    mut fire_mode: ResMut<SelectedFireMode>,
) {
    // Only react when the selection actually changed this update.
    if !selected.is_changed() {
        return;
    }
    let Some(shooter) = **selected else {
        // Selection cleared — leave the mode (it rides until the next armed select).
        return;
    };
    let Ok(weapon) = weapons.get(shooter) else {
        // Selected an unarmed entity — no FireMode to default to.
        return;
    };
    let next = SelectedFireMode::new(weapon.single());
    if *fire_mode != next {
        *fire_mode = next;
    }
}
