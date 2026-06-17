//! The selected **fire mode** (GTW-227 / GTW-48 S8 222b): the [`SelectedFireMode`]
//! resource the FIRE surface feeds into [`FireRequested`](gdtf_battle_sim::acts::FireRequested),
//! and the offered-mode cycle the fire-mode-cycle key steps.
//!
//! The selected mode is the per-mode numbers ([`FireModeSpec`]) the next shot fires
//! with. On selecting an armed ganger it is set to that weapon's
//! [`FireMode::single`](gdtf_battle_sim::FireMode::single) (the `Single`-kind mode);
//! the fire-mode-cycle key advances it among ONLY the modes the selected weapon's
//! [`FireMode`](gdtf_battle_sim::FireMode) selector offers — never inventing a mode
//! the weapon does not author. The stepping walks the selector's LIST by
//! [`ModeKind`](gdtf_battle_sim::ModeKind) identity, so a one-mode weapon stays on
//! that mode, a two-mode weapon toggles, and a three-mode weapon walks the list and
//! wraps.

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

/// The [`FireModeSpec`] AFTER `current` in the selected weapon's offered list,
/// wrapping past the last offered mode back to the first.
///
/// Walks the selector's LIST — enumerating ONLY the modes the weapon authors
/// (resolution.md §1: the selector is authored per weapon) — and identifies a mode by
/// its closed [`ModeKind`](gdtf_battle_sim::ModeKind) (no float compare). It NEVER
/// invents a mode a weapon does not offer:
///
/// - an EMPTY selector returns `current` (defensive, no panic);
/// - a one-mode weapon's only mode is also `current`'s kind → wraps to index 0 (stays
///   on it);
/// - a two-mode weapon toggles its two modes;
/// - a three-mode weapon walks the list and wraps to the first.
///
/// `current` is matched against the offered specs by KIND to locate the current
/// position; a `current.kind` that is NOT offered (e.g. a stale mode from a
/// previously-selected weapon) restarts the list one past the first —
/// [`get(1)`](slice::get) if present, else index `0` — a total, never-panicking
/// fallback (so a stale current steps to the 2nd mode on a 2+-mode weapon, else stays
/// on the only mode of a 1-mode weapon).
#[must_use]
pub fn next_fire_mode(current: FireModeSpec, selector: &FireMode) -> FireModeSpec {
    // `selector` derefs to `&[FireModeSpec]`. An empty selector has nothing to walk —
    // return `current` unchanged (defensive).
    if selector.is_empty() {
        return current;
    }
    let len = selector.len();
    // Find the offered mode whose kind matches `current.kind` and advance one,
    // wrapping. `FireModeSpec` is `Copy` again, so the chosen mode is `.copied()` out.
    if let Some(i) = selector.iter().position(|spec| spec.kind == current.kind) {
        return selector.get((i + 1) % len).copied().unwrap_or(current);
    }
    // `current.kind` is not offered — restart one past the first: the 2nd mode if the
    // weapon offers one, else the only mode.
    selector
        .get(1)
        .or_else(|| selector.first())
        .copied()
        .unwrap_or(current)
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

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{ModeConeMult, ModeKind, ModeShots, ModeTuPercent};

    use super::*;

    /// A fire-mode spec of an explicit [`ModeKind`](gdtf_battle_sim::ModeKind) — the
    /// kind is what the walk identifies a mode by (the cone/TU/shots magnitudes are
    /// arbitrary, not pinned tuning).
    const fn spec(kind: ModeKind, tu_percent: f32, shots: u16) -> FireModeSpec {
        FireModeSpec::new(
            kind,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(tu_percent),
            ModeShots::new(shots),
        )
    }

    /// AC3 — a one-mode `[Single]` weapon stays on `single` across repeated cycles
    /// (no other mode is invented).
    #[test]
    fn single_weapon_stays_on_single() {
        let single = spec(ModeKind::Single, 0.2, 1);
        let selector = FireMode::new(vec![single]);
        let mut current = single;
        for _ in 0..5 {
            current = next_fire_mode(current, &selector);
            assert_eq!(current, single, "a one-mode weapon must stay on single");
        }
    }

    /// AC3 — a two-mode `[Single, Burst]` weapon toggles single <-> burst.
    #[test]
    fn single_burst_weapon_toggles_single_and_burst() {
        let single = spec(ModeKind::Single, 0.2, 1);
        let burst = spec(ModeKind::Burst, 0.4, 3);
        let selector = FireMode::new(vec![single, burst]);

        // single -> burst -> single -> burst ...
        let after_single = next_fire_mode(single, &selector);
        assert_eq!(after_single, burst, "single must advance to burst");
        let after_burst = next_fire_mode(burst, &selector);
        assert_eq!(after_burst, single, "burst must wrap back to single");
    }

    /// AC3 — a three-mode `[Single, Burst, Full]` weapon walks `single` -> `burst` ->
    /// `full` -> (wrap) `single`.
    #[test]
    fn full_auto_weapon_walks_all_three_and_wraps() {
        let single = spec(ModeKind::Single, 0.2, 1);
        let burst = spec(ModeKind::Burst, 0.4, 3);
        let full = spec(ModeKind::Full, 0.7, 6);
        let selector = FireMode::new(vec![single, burst, full]);

        let a = next_fire_mode(single, &selector);
        assert_eq!(a, burst, "single -> burst");
        let b = next_fire_mode(a, &selector);
        assert_eq!(b, full, "burst -> full");
        let c = next_fire_mode(b, &selector);
        assert_eq!(c, single, "full wraps back to single");
    }

    /// AC3 — a stale `current.kind` (not offered by the selected weapon) restarts the
    /// list one past the first (defensive — a previously-selected weapon's mode never
    /// sticks the cycle). On a two-mode weapon it steps to the 2nd mode.
    #[test]
    fn unknown_current_restarts_one_past_first() {
        let single = spec(ModeKind::Single, 0.2, 1);
        let burst = spec(ModeKind::Burst, 0.4, 3);
        // A `Full`-kind current, but the weapon offers only single + burst.
        let stale = spec(ModeKind::Full, 9.9, 99);
        let selector = FireMode::new(vec![single, burst]);
        assert_eq!(
            next_fire_mode(stale, &selector),
            burst,
            "a stale current must restart the list at the 2nd mode (burst)",
        );
    }
}
