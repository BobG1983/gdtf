//! The selected **fire mode** (GTW-227 / GTW-48 S8 222b): the [`SelectedFireMode`]
//! resource the FIRE surface feeds into [`FireRequested`](gdtf_battle_sim::acts::FireRequested),
//! and the offered-mode cycle the fire-mode-cycle key steps.
//!
//! The selected mode is the per-mode numbers ([`FireModeSpec`]) the next shot fires
//! with. On selecting an armed ganger it is set to that weapon's
//! [`FireMode::single`](gdtf_battle_sim::FireMode::single) (the base mode present on
//! every selector variant); the fire-mode-cycle key advances it among ONLY the modes
//! the selected weapon's [`FireMode`](gdtf_battle_sim::FireMode) selector offers —
//! never inventing a mode the weapon does not author. The stepping reads the selector
//! VARIANT (single / single+burst / single+burst+full-auto), so a `Single` weapon
//! stays on single, a `SingleBurst` weapon toggles single<->burst, and a
//! `SingleBurstFullAuto` weapon walks single->burst->full_auto->single.

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
/// [`FireMode::single`](gdtf_battle_sim::FireMode::single), and advanced by the
/// fire-mode-cycle drain arm.
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
    /// The structural single-shot default — `cone_mult = 1`, `tu_percent = 0`,
    /// `shots = 1`. A spawn-time placeholder, NOT a balance value: the selection path
    /// resets it to the picked weapon's authored [`FireMode::single`] the moment an
    /// armed ganger is selected, so this default is only ever live before any
    /// selection.
    fn default() -> Self {
        Self(FireModeSpec::new(
            gdtf_battle_sim::ModeConeMult::new(1.0),
            gdtf_battle_sim::ModeTuPercent::new(0.0),
            gdtf_battle_sim::ModeShots::new(1),
        ))
    }
}

/// The [`FireModeSpec`] AFTER `current` in the selected weapon's offered ladder,
/// wrapping past the last offered mode back to single.
///
/// Reads the selector VARIANT to enumerate ONLY the modes the weapon authors
/// (resolution.md §1: "selector is single / single+burst / single+burst+full-auto,
/// authored per weapon") — it NEVER invents a mode a weapon does not offer:
///
/// - [`FireMode::Single`] offers only `single`, so the cycle stays on `single`.
/// - [`FireMode::SingleBurst`] toggles `single` <-> `burst`.
/// - [`FireMode::SingleBurstFullAuto`] walks `single` -> `burst` -> `full_auto` ->
///   (wrap) `single`.
///
/// `current` is matched against the weapon's authored specs to locate the current
/// rung; a `current` that is not one of the offered specs (e.g. a stale mode from a
/// previously-selected weapon) restarts the ladder at the rung AFTER `single` — i.e.
/// it steps to `burst` when offered, else stays `single` — a total, never-panicking
/// fallback.
#[must_use]
pub fn next_fire_mode(current: FireModeSpec, selector: &FireMode) -> FireModeSpec {
    match *selector {
        // Single-only: there is nothing to advance to — stay on single.
        FireMode::Single { single } => single,
        // Single + burst: toggle. On `burst` (or any non-burst rung) wrap to single
        // when already on burst, else advance to burst.
        FireMode::SingleBurst { single, burst } => {
            if specs_eq(current, burst) {
                single
            } else {
                burst
            }
        }
        // Single + burst + full-auto: walk single -> burst -> full_auto -> single.
        FireMode::SingleBurstFullAuto {
            single,
            burst,
            full_auto,
        } => {
            if specs_eq(current, single) {
                burst
            } else if specs_eq(current, burst) {
                full_auto
            } else if specs_eq(current, full_auto) {
                single
            } else {
                // A stale / unknown current spec restarts the ladder one past single.
                burst
            }
        }
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

/// Whether two [`FireModeSpec`]s are the same authored mode.
///
/// The specs are `Copy` and never mutated after authoring, so a direct
/// [`PartialEq`] is the rung comparison (the authored values flow unchanged from the
/// weapon's [`FireMode`] selector through [`SelectedFireMode`]). Factored out so the
/// [`next_fire_mode`] ladder reads `specs_eq(current, burst)` rather than an inline
/// `==` that clippy's float-comparison lint would flag at each call site.
fn specs_eq(a: FireModeSpec, b: FireModeSpec) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{ModeConeMult, ModeShots, ModeTuPercent};

    use super::*;

    /// A fire-mode spec with a marker `tu_percent` so the three rungs are distinct
    /// authored values (the magnitudes are arbitrary, not pinned tuning).
    fn spec(tu_percent: f32, shots: u16) -> FireModeSpec {
        FireModeSpec::new(
            ModeConeMult::new(1.0),
            ModeTuPercent::new(tu_percent),
            ModeShots::new(shots),
        )
    }

    /// AC2 — a `Single` weapon stays on `single` across repeated cycles (no other
    /// mode is invented).
    #[test]
    fn single_weapon_stays_on_single() {
        let single = spec(0.2, 1);
        let selector = FireMode::Single { single };
        let mut current = single;
        for _ in 0..5 {
            current = next_fire_mode(current, &selector);
            assert_eq!(current, single, "a Single weapon must stay on single");
        }
    }

    /// AC2 — a `SingleBurst` weapon toggles single <-> burst.
    #[test]
    fn single_burst_weapon_toggles_single_and_burst() {
        let single = spec(0.2, 1);
        let burst = spec(0.4, 3);
        let selector = FireMode::SingleBurst { single, burst };

        // single -> burst -> single -> burst ...
        let after_single = next_fire_mode(single, &selector);
        assert_eq!(after_single, burst, "single must advance to burst");
        let after_burst = next_fire_mode(burst, &selector);
        assert_eq!(after_burst, single, "burst must wrap back to single");
    }

    /// AC2 — a `SingleBurstFullAuto` weapon walks `single` -> `burst` -> `full_auto`
    /// -> (wrap) `single`.
    #[test]
    fn full_auto_weapon_walks_all_three_and_wraps() {
        let single = spec(0.2, 1);
        let burst = spec(0.4, 3);
        let full_auto = spec(0.7, 6);
        let selector = FireMode::SingleBurstFullAuto {
            single,
            burst,
            full_auto,
        };

        let a = next_fire_mode(single, &selector);
        assert_eq!(a, burst, "single -> burst");
        let b = next_fire_mode(a, &selector);
        assert_eq!(b, full_auto, "burst -> full_auto");
        let c = next_fire_mode(b, &selector);
        assert_eq!(c, single, "full_auto wraps back to single");
    }

    /// A stale / unknown current spec restarts the ladder one rung past single
    /// (defensive — a previously-selected weapon's mode never sticks the cycle).
    #[test]
    fn unknown_current_restarts_one_past_single() {
        let single = spec(0.2, 1);
        let burst = spec(0.4, 3);
        let stale = spec(9.9, 99);
        let selector = FireMode::SingleBurst { single, burst };
        assert_eq!(
            next_fire_mode(stale, &selector),
            burst,
            "a stale current must restart the ladder at burst",
        );
    }
}
