//! The **posture / orientation verbs** — the E4.1 mutators a ganger spends a turn on:
//! [`set_aiming`] toggles the aim flag, [`set_stance`] changes posture, and
//! [`set_facing`] turns in place. Each verb is **pure** — it takes and mutates the
//! landed ganger components ([`Aiming`] / [`Stance`] / [`Facing`]) plus the ganger's
//! [`Tu`] pool, reads the relevant tuning leaf, and charges through the E4.0
//! [`crate::tu::spend_tu`] — there is **no [`World`](bevy::ecs::world::World) access**,
//! so the verbs unit-test against bare component values with no ECS plumbing.
//!
//! Whether each verb charges TU is **grounded in the docs**, not invented:
//!
//! - [`set_aiming`] charges **no TU** — `docs/combat/combat.md` L34's action list
//!   (step, turn, snap/aimed/auto shot, kneel) does NOT name toggling the aim FLAG as a
//!   costed action; the aim cost is the **fire-time ×1.5 shot-cost premium**
//!   (`docs/combat/resolution.md` §1a, the [`crate::tuning::AimTuPremium`] leaf),
//!   charged when the shot fires (E4.2/E4.5), not per toggle. So it is a pure flag
//!   setter.
//! - [`set_stance`] charges [`crate::tuning::StanceChangeTu`] — combat.md L34 lists
//!   "kneel" among the actions that cost TUs and resolution.md §"What's tunable" names
//!   "stance-change TU".
//! - [`set_facing`] charges [`crate::tuning::TurnTu`] — combat.md L34 affirmatively
//!   lists "turn" among the costed actions; only the *magnitude* is docs-unspecified
//!   (a value-agnostic tuning leaf, mirroring the stance-change-TU precedent). Turning
//!   is **not** a free toggle.
//!
//! Both costed verbs charge **only when the value actually changes** — re-asserting a
//! stance / facing a ganger already holds is a no-op (you don't pay to not move). The
//! charge is **saturating** (it floors at `0`, never underflows) because it goes
//! through [`crate::tu::spend_tu`]. The COST magnitudes are tuning (`tuning.ron`); this
//! slice ships the mechanism.

use crate::{
    ganger::{Aiming, Direction, Facing, Stance, StanceKind, Tu},
    tu::spend_tu,
    tuning::{StanceChangeTu, TurnTu},
};

/// Set a ganger's [`Aiming`] flag — **charges no TU**.
///
/// A pure setter of the aim-mode flag: `*aiming = Aiming::new(on)`. Toggling aim is
/// NOT a costed action (`docs/combat/combat.md` L34's action list does not name it);
/// the aim cost is the **fire-time ×1.5 shot-cost premium** (`docs/combat/resolution.md`
/// §1a, the [`crate::tuning::AimTuPremium`] leaf), charged when the shot fires, not per
/// toggle. So no [`Tu`] is touched here — this verb takes no TU pool at all.
pub const fn set_aiming(aiming: &mut Aiming, on: bool) {
    *aiming = Aiming::new(on);
}

/// Change a ganger's [`Stance`] to `to`, charging [`StanceChangeTu`] **only on a real
/// change** — returns whether the stance changed.
///
/// If the ganger already holds posture `to`, this is a **no-op**: the stance is
/// unchanged and **no TU is spent** (you don't pay to not move), and it returns
/// `false`. Otherwise it spends `cost` from `tu` via [`crate::tu::spend_tu`] (saturating
/// — it floors at `0`, never underflows) and sets the new stance, returning `true`.
///
/// The charge is grounded in `docs/combat/combat.md` L34 ("kneel" costs TUs) and
/// `docs/combat/resolution.md` §"What's tunable" (stance-change TU); the magnitude is
/// the [`StanceChangeTu`] tuning leaf, not a hardcoded constant.
pub fn set_stance(stance: &mut Stance, tu: &mut Tu, to: StanceKind, cost: &StanceChangeTu) -> bool {
    if **stance == to {
        // Re-asserting the posture the ganger already holds — a no-op, no charge.
        return false;
    }
    spend_tu(tu, Tu::new(**cost));
    *stance = Stance::new(to);
    true
}

/// Turn a ganger to face [`Direction`] `to`, charging [`TurnTu`] **only on a real
/// change** — returns whether the facing changed.
///
/// If the ganger already faces `to`, this is a **no-op**: the facing is unchanged and
/// **no TU is spent** (you don't pay to not turn), and it returns `false`. Otherwise it
/// spends `cost` from `tu` via [`crate::tu::spend_tu`] (saturating — it floors at `0`,
/// never underflows) and sets the new facing, returning `true`.
///
/// Turning **costs** TU: `docs/combat/combat.md` L34 affirmatively lists "turn" among
/// the actions that cost TUs. Only the *magnitude* is docs-unspecified — it is the
/// [`TurnTu`] tuning leaf (value-agnostic, mirroring [`StanceChangeTu`]), NOT a free
/// toggle.
pub fn set_facing(facing: &mut Facing, tu: &mut Tu, to: Direction, cost: &TurnTu) -> bool {
    if **facing == to {
        // Re-asserting the facing the ganger already holds — a no-op, no charge.
        return false;
    }
    spend_tu(tu, Tu::new(**cost));
    *facing = Facing::new(to);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC1 — set_aiming toggles the Aiming flag AND leaves the Tu pool unchanged (no
    // charge): the aim cost is the fire-time ×1.5 premium, not a per-toggle charge. The
    // verb takes no Tu at all, so the test holds an unrelated pool and proves it is
    // untouched across both toggle directions.

    #[test]
    fn set_aiming_toggles_flag_and_charges_no_tu() {
        let mut aiming = Aiming::new(false);
        // A pool the verb must never touch (set_aiming takes no Tu — this guards the
        // contract that toggling aim is free).
        let tu = Tu::new(30);

        set_aiming(&mut aiming, true);
        assert!(*aiming, "set_aiming(true) must set the aim flag");
        assert_eq!(*tu, 30, "toggling aim must not spend any TU");

        set_aiming(&mut aiming, false);
        assert!(!*aiming, "set_aiming(false) must clear the aim flag");
        assert_eq!(*tu, 30, "toggling aim off must not spend any TU either");
    }

    // AC2 — set_stance to a DIFFERENT stance sets the new Stance AND drops Tu by exactly
    // StanceChangeTu (relation: strictly decreased, drop == *cost). Value-agnostic on
    // the cost magnitude (a chosen test cost, asserted only by its relation to the drop).

    #[test]
    fn set_stance_to_different_stance_spends_exactly_the_cost() {
        let cost = StanceChangeTu::new(8);
        let mut stance = Stance::new(StanceKind::Standing);
        let mut tu = Tu::new(60);
        let before = *tu;

        let changed = set_stance(&mut stance, &mut tu, StanceKind::Prone, &cost);

        assert!(
            changed,
            "changing to a different stance must report a change"
        );
        assert_eq!(
            *stance,
            StanceKind::Prone,
            "the new stance must be set after a real change",
        );
        assert!(
            *tu < before,
            "a real stance change must strictly decrease TU"
        );
        assert_eq!(
            before - *tu,
            *cost,
            "the TU drop must equal exactly the StanceChangeTu tuning leaf",
        );
    }

    // AC3 — set_stance to the SAME stance is a no-op on TU (and on the stance): you don't
    // pay to not move. Asserts the pool is unchanged and the verb reports no change.

    #[test]
    fn set_stance_to_same_stance_is_a_no_op() {
        let cost = StanceChangeTu::new(8);
        let mut stance = Stance::new(StanceKind::Crouching);
        let mut tu = Tu::new(60);

        let changed = set_stance(&mut stance, &mut tu, StanceKind::Crouching, &cost);

        assert!(
            !changed,
            "re-asserting the held stance must report no change"
        );
        assert_eq!(
            *stance,
            StanceKind::Crouching,
            "the stance is unchanged on a no-op",
        );
        assert_eq!(
            *tu, 60,
            "re-asserting the held stance must not spend any TU"
        );
    }

    // AC4 — set_facing to a DIFFERENT facing sets the new Facing AND drops Tu by exactly
    // TurnTu (relation only — value-agnostic on the magnitude).

    #[test]
    fn set_facing_to_different_facing_spends_exactly_the_cost() {
        let cost = TurnTu::new(4);
        let mut facing = Facing::new(Direction::North);
        let mut tu = Tu::new(50);
        let before = *tu;

        let changed = set_facing(&mut facing, &mut tu, Direction::East, &cost);

        assert!(
            changed,
            "turning to a different facing must report a change"
        );
        assert_eq!(
            *facing,
            Direction::East,
            "the new facing must be set after a real turn",
        );
        assert!(*tu < before, "a real turn must strictly decrease TU");
        assert_eq!(
            before - *tu,
            *cost,
            "the TU drop must equal exactly the TurnTu tuning leaf",
        );
    }

    // AC5 — set_facing to the SAME facing is a no-op on TU (and on the facing): you don't
    // pay to not turn.

    #[test]
    fn set_facing_to_same_facing_is_a_no_op() {
        let cost = TurnTu::new(4);
        let mut facing = Facing::new(Direction::SouthWest);
        let mut tu = Tu::new(50);

        let changed = set_facing(&mut facing, &mut tu, Direction::SouthWest, &cost);

        assert!(
            !changed,
            "re-asserting the held facing must report no change"
        );
        assert_eq!(
            *facing,
            Direction::SouthWest,
            "the facing is unchanged on a no-op",
        );
        assert_eq!(
            *tu, 50,
            "re-asserting the held facing must not spend any TU"
        );
    }

    // The charge goes through the saturating spend_tu — a stance change that costs more
    // than the pool holds floors the pool at 0 (no underflow wrap), while still applying
    // the new stance. Proves the verb inherits E4.0's saturating discipline.

    #[test]
    fn set_stance_charge_saturates_when_cost_exceeds_pool() {
        let cost = StanceChangeTu::new(200);
        let mut stance = Stance::new(StanceKind::Standing);
        let mut tu = Tu::new(5);

        let changed = set_stance(&mut stance, &mut tu, StanceKind::Prone, &cost);

        assert!(
            changed,
            "the stance still changes even when TU cannot cover it"
        );
        assert_eq!(*stance, StanceKind::Prone, "the new stance is applied");
        assert_eq!(*tu, 0, "over-spending floors the pool at 0, never wraps");
    }
}
