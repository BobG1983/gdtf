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
//! - [`set_facing`] charges [`crate::tuning::TurnTu`] **per 45deg step** — combat.md L34
//!   affirmatively lists "turn" among the costed actions; the per-step magnitude is a
//!   value-agnostic tuning leaf (USER DECISION: 1 TU/step). It is a **PARTIAL** turn: the
//!   ganger turns as many whole 45deg steps as its [`Tu`] pool affords and lands partway
//!   when it runs out (0 affordable steps = no turn, no charge). Turning is **not** a free
//!   toggle.
//!
//! Both costed verbs charge **only when the value actually changes** — re-asserting a
//! stance / facing a ganger already holds is a no-op (you don't pay to not move). The
//! charge is **saturating** (it floors at `0`, never underflows) because it goes
//! through [`crate::tu::spend_tu`]; [`set_facing`]'s per-step charge is also EXACT by
//! construction (it never exceeds the pool). The COST magnitudes are tuning
//! (`tuning.ron`); this slice ships the mechanism.

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

/// Turn a ganger toward [`Direction`] `to`, charging **one [`TurnTu`] per 45deg step**
/// and turning only as far as the [`Tu`] pool affords — the **PARTIAL-TURN** model.
/// Returns whether the facing changed.
///
/// `cost` is the per-step price (`TurnTu` per 45deg step). The full short-way rotation is
/// `total = (**facing).steps_to(to)` steps; the ganger turns the `afford` whole steps its
/// pool pays for — `afford = min(total, pool / per)` (treating `per == 0` as fully
/// affordable) — landing on `to` when it can pay for the whole turn, otherwise **partway**
/// at `(**facing).rotated_toward(to, afford)`. The charge is **exact**: `per * afford`,
/// which by construction never exceeds the held pool (it still goes through the one
/// saturating [`crate::tu::spend_tu`] path).
///
/// Two no-turn cases return `false` and charge **nothing**: re-asserting the held facing
/// (`steps_to == 0` — you don't pay to not turn) and a pool that cannot afford even one
/// step (`afford == 0`, i.e. `pool < per`). Otherwise it sets the (possibly intermediate)
/// facing and returns `true`.
///
/// Turning **costs** TU: `docs/combat/combat.md` L34 affirmatively lists "turn" among the
/// actions that cost TUs; the per-step magnitude is the [`TurnTu`] tuning leaf
/// (value-agnostic, USER DECISION: `1` per step). NOT a free toggle.
pub fn set_facing(facing: &mut Facing, tu: &mut Tu, to: Direction, cost: &TurnTu) -> bool {
    // The full short-way rotation, in whole 45deg steps. Zero == already facing `to`.
    let total = (**facing).steps_to(to);
    if total == 0 {
        // Re-asserting the facing the ganger already holds — a no-op, no charge.
        return false;
    }
    let per = **cost;
    // How many WHOLE steps the pool pays for, capped at the full rotation. A zero
    // per-step cost is treated as fully affordable: `checked_div` yields `None` on a
    // zero divisor, which we read as "the whole rotation is free" (guards divide-by-zero).
    let pool_steps = (**tu).checked_div(per).unwrap_or(total);
    let afford = if pool_steps < total {
        pool_steps
    } else {
        total
    };
    if afford == 0 {
        // The pool cannot afford even one step (pool < per) — no turn, no charge.
        return false;
    }
    // Compute the landing facing BEFORE mutating; lands at `to` iff afford == total.
    let landing = (**facing).rotated_toward(to, afford);
    *facing = Facing::new(landing);
    // Exact charge: per * afford never exceeds the pool by construction.
    spend_tu(tu, Tu::new(per * afford));
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

    // AC4 — a FULLY-affordable turn N steps away charges exactly N * TurnTu (one per
    // 45deg step) and lands on `to`. Relation only — value-agnostic on the magnitude.
    // North -> East is a 2-step short-way turn; North -> South (opposite) is 4 steps.

    #[test]
    fn set_facing_to_different_facing_spends_exactly_the_cost() {
        let cost = TurnTu::new(4);

        // A 2-step turn on an AMPLE pool: drop == steps_to * cost, lands on `to`.
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
            "a fully-affordable turn lands on the requested facing",
        );
        assert!(*tu < before, "a real turn must strictly decrease TU");
        assert_eq!(
            before - *tu,
            Direction::North.steps_to(Direction::East) * (*cost),
            "the TU drop must equal exactly (short-way steps) * the per-step TurnTu leaf",
        );

        // An opposite (4-step) turn on a fresh ample pool: drop == 4 * cost, lands on `to`.
        let mut facing = Facing::new(Direction::North);
        let mut tu = Tu::new(50);
        let before = *tu;

        let changed = set_facing(&mut facing, &mut tu, Direction::South, &cost);

        assert!(
            changed,
            "an opposite turn on an ample pool reports a change"
        );
        assert_eq!(
            *facing,
            Direction::South,
            "a fully-affordable opposite turn lands on the requested facing",
        );
        assert_eq!(
            before - *tu,
            Direction::North.steps_to(Direction::South) * (*cost),
            "an opposite turn costs steps_to (= 4) * the per-step TurnTu leaf",
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

    // AC7 — PARTIAL turn: an under-affordable multi-step turn turns AS FAR AS AFFORDED,
    // lands partway, and spends EXACTLY afford * cost. With cost = C and a pool of 2*C,
    // only 2 of a 4-step North -> South turn are affordable, so it turns 2 steps and lands
    // at North.rotated_toward(South, 2) (= East, the 90° partway facing). Relation only.

    #[test]
    fn set_facing_partial_turn_spends_exactly_the_afforded_steps() {
        // A chosen per-step cost C (the magnitude is tuning — only the relation matters).
        let c = 7u8;
        let cost = TurnTu::new(c);
        let mut facing = Facing::new(Direction::North);
        // A pool that affords exactly 2 whole steps of the 4-step opposite turn.
        let mut tu = Tu::new(2 * c);
        let before = *tu;

        let changed = set_facing(&mut facing, &mut tu, Direction::South, &cost);

        assert!(
            changed,
            "a partly-affordable turn still turns (returns true)"
        );
        assert_eq!(
            *facing,
            Direction::North.rotated_toward(Direction::South, 2),
            "an under-affordable turn lands partway at the afforded short-way facing",
        );
        // The partway facing is the 90°-short East (sanity on the rotated_toward landing).
        assert_eq!(
            *facing,
            Direction::East,
            "two short-way steps from North toward South land on East",
        );
        assert_eq!(
            before - *tu,
            2 * (*cost),
            "the charge is exactly the afforded steps (= 2) * the per-step cost",
        );
        assert_eq!(
            *tu, 0,
            "the 2-step charge drains the 2-step pool to exactly 0"
        );
    }

    // AC8 — ZERO affordable steps: a turn the ganger cannot pay one step of is NO turn and
    // NO charge. With cost = C (C >= 2) and a pool of C - 1 (< cost, cannot afford even one
    // step), set_facing returns false, the facing is unchanged, and the pool is untouched.

    #[test]
    fn set_facing_with_pool_below_one_step_does_not_turn_or_charge() {
        let c = 5u8; // C >= 2 so C - 1 is a positive, sub-one-step pool.
        let cost = TurnTu::new(c);
        let mut facing = Facing::new(Direction::North);
        let mut tu = Tu::new(c - 1);

        let changed = set_facing(&mut facing, &mut tu, Direction::South, &cost);

        assert!(
            !changed,
            "a pool below one step's cost must report no turn (false)"
        );
        assert_eq!(
            *facing,
            Direction::North,
            "the facing is unchanged when not even one step is affordable",
        );
        assert_eq!(
            *tu,
            c - 1,
            "no charge is taken when not even one step is affordable",
        );

        // The Tu::new(0) sub-case: a broke ganger likewise cannot turn and is not charged.
        let mut facing = Facing::new(Direction::North);
        let mut tu = Tu::new(0);
        let changed = set_facing(&mut facing, &mut tu, Direction::South, &cost);
        assert!(!changed, "a broke ganger (Tu == 0) cannot turn");
        assert_eq!(
            *facing,
            Direction::North,
            "a broke ganger's facing is unchanged"
        );
        assert_eq!(*tu, 0, "a broke ganger is not charged");
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
