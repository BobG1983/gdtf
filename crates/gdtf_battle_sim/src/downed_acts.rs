//! The §9 from-Downed verbs + their shared faction-aware predicates — the E3.8
//! slice (GTW-190).
//!
//! Two adjacency verbs act on a [`LifeState::Downed`] ganger, and each shares its
//! guard with a predicate the HUD button and the act both call (one guard set, so
//! the button is enabled **exactly** when the act would succeed —
//! `docs/combat/resolution.md` §9: "enabled off the model's own `can_stabilize`
//! predicate so button and act share one guard set"):
//!
//! - [`stabilize_downed`] — an 8-adjacent ALIVE **ally** dresses the wound, halting
//!   the bleed-out clock: it **sets** the E3.7 [`Stabilized`] flag (defined in
//!   `ganger.rs`, reused here — this slice only *sets* it; [`crate::bleed::tick_bleed`]
//!   *reads* it). The ganger **remains [`LifeState::Downed`]** — NOT Alive, NOT Dead.
//! - [`execute_downed`] — an 8-adjacent ALIVE **enemy** finishes the Downed ganger
//!   outright → [`LifeState::Dead`].
//!
//! ## The faction differentiator
//!
//! `Faction` is wired INTO both predicates — it is what tells an ally apart from an
//! enemy (`docs/combat/wounds-and-roster.md` §"Downed → death … state machine":
//! stabilize is an **ally** action, execute an **enemy** one):
//!
//! - [`can_stabilize`] requires `actor.faction == target.faction` (an **ally**) — an
//!   enemy can **never** stabilize.
//! - [`can_execute`] requires `actor.faction != target.faction` (an **enemy**) — an
//!   ally can **never** execute.
//!
//! ## 8-adjacency (same-level Moore-8)
//!
//! [`is_8_adjacent`] is the literal "8 surrounding cells": the same [`Level`]
//! (`z` equal) **and** Chebyshev distance 1 in the `x`/`y` cell plane
//! (`max(|dx|, |dy|) == 1`), excluding the same cell. An actor a storey above or
//! below is **not** adjacent — reach is the 8 same-storey neighbours, not the 26
//! cross-level voxels (`docs/combat/resolution.md` §9: an "8-adjacent" actor).
//!
//! ## TU boundary (E4)
//!
//! The flat [`crate::tuning::StabilizeTu`] / [`crate::tuning::ExecuteTu`] costs are
//! **READ** from tuning to wire the leaf (each act returns the cost it consulted),
//! but this slice runs **no TU economy**: it never debits a [`crate::ganger::Tu`]
//! pool nor checks can-afford — that is **E4**. Pure, render-free model logic: the
//! predicates operate on component values, the acts mutate component references; no
//! renderer, no pixel.

use crate::{
    ganger::{Faction, LifeState, Position, Stabilized},
    tuning::{CombatTuning, ExecuteTu, StabilizeTu},
};

/// Whether two grid [`Position`]s are **8-adjacent** — the same-level Moore-8
/// neighbourhood (the 8 surrounding cells on the *same* storey).
///
/// True when the two positions share a [`Level`](crate::metric::Level) (their `z`
/// storey indices are equal) **and** their cells are Chebyshev-distance 1 apart on
/// the `x`/`y` ground
/// plane (`max(|dx|, |dy|) == 1`), which excludes the same cell. An actor on a
/// different storey is **not** adjacent (this is same-level Moore-8, not the 26-cell
/// cross-level reach) — the literal reading of "8-adjacent" in
/// `docs/combat/resolution.md` §9. Pure integer arithmetic over the cubic-voxel
/// metric — no pixel.
#[must_use]
pub fn is_8_adjacent(a: Position, b: Position) -> bool {
    // The (cell, level) keys: x/y are cells, z is the storey index (metric.rs).
    let pa = **a;
    let pb = **b;
    // Same storey: a ganger above or below is NOT in the 8 surrounding cells.
    if pa.z != pb.z {
        return false;
    }
    let dx = (pa.x - pb.x).abs();
    let dy = (pa.y - pb.y).abs();
    // Chebyshev distance exactly 1: the 8 cells ringing the actor, excluding itself
    // (dx == 0 && dy == 0 → the same cell, not adjacent).
    dx <= 1 && dy <= 1 && (dx != 0 || dy != 0)
}

/// The acting ganger's reads for a from-Downed predicate — a small named bundle so
/// the predicates stay under clippy's 8-argument gate and read cleanly (the
/// [`crate::resolve_coarse::ShotInputs`] / [`crate::severity::SeverityInputs`]
/// precedent).
///
/// A transparent argument record of the existing named ganger components (each a
/// `Copy` E1 newtype) — not itself a wrapped domain scalar, so it is passed by
/// value. The actor is the would-be stabilizer / executor; only its position, life
/// state, and faction gate the act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    /// The actor's grid [`Position`] — tested for 8-adjacency to the target.
    pub pos:     Position,
    /// The actor's [`LifeState`] — only an [`LifeState::Alive`] actor may act.
    pub life:    LifeState,
    /// The actor's [`Faction`] — the differentiator: equal to the target's faction
    /// gates **stabilize** (ally), unequal gates **execute** (enemy).
    pub faction: Faction,
}

/// The downed target's reads for a from-Downed predicate — a small named bundle
/// (the same precedent as [`Actor`]).
///
/// A transparent argument record of the existing named ganger components (each a
/// `Copy` E1 newtype). The target is the would-be-stabilized / -executed ganger;
/// the act applies only to a [`LifeState::Downed`] one, and stabilize additionally
/// requires it is not already [`Stabilized`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownedTarget {
    /// The target's grid [`Position`] — tested for 8-adjacency to the actor.
    pub pos:        Position,
    /// The target's [`LifeState`] — only a [`LifeState::Downed`] target is a valid
    /// subject for either act.
    pub life:       LifeState,
    /// The target's [`Faction`] — compared against the actor's (ally vs enemy).
    pub faction:    Faction,
    /// The target's [`Stabilized`] flag, if present — stabilize is rejected when the
    /// target is **already** stabilized (`Some(true)`); absent or `Some(false)` is
    /// not-yet-stabilized. Unused by [`can_execute`] (an executable ganger may be
    /// stabilized or not).
    pub stabilized: Option<Stabilized>,
}

/// Whether `actor` can **stabilize** `target` — the shared guard for the HUD
/// Stabilize button and [`stabilize_downed`] (`docs/combat/resolution.md` §9).
///
/// True iff **all** hold:
///
/// 1. the two are [`is_8_adjacent`] (same-level Moore-8 reach),
/// 2. the actor is [`LifeState::Alive`] (a downed / dead actor cannot act),
/// 3. the target is [`LifeState::Downed`] (you stabilize the downed, not the
///    standing or the dead),
/// 4. `actor.faction == target.faction` — an **ally** (the faction differentiator;
///    an enemy can never stabilize), and
/// 5. the target is **not already stabilized** (its [`Stabilized`] is absent or
///    `Some(false)`; re-dressing a halted clock is a no-op).
///
/// Pure read over component values — no mutation, no draw, no pixel.
#[must_use]
pub fn can_stabilize(actor: &Actor, target: &DownedTarget) -> bool {
    is_8_adjacent(actor.pos, target.pos)
        && actor.life == LifeState::Alive
        && target.life == LifeState::Downed
        && actor.faction == target.faction
        && !target.stabilized.is_some_and(|s| *s)
}

/// Whether `actor` can **execute** `target` — the shared guard for the HUD Execute
/// button and [`execute_downed`] (`docs/combat/resolution.md` §9).
///
/// True iff **all** hold:
///
/// 1. the two are [`is_8_adjacent`] (same-level Moore-8 reach),
/// 2. the actor is [`LifeState::Alive`],
/// 3. the target is [`LifeState::Downed`], and
/// 4. `actor.faction != target.faction` — an **enemy** (the faction differentiator;
///    an ally can never execute).
///
/// The target's [`Stabilized`] flag does **not** gate execute — a stabilized Downed
/// ganger can still be finished off by an enemy. Pure read over component values —
/// no mutation, no draw, no pixel.
#[must_use]
pub fn can_execute(actor: &Actor, target: &DownedTarget) -> bool {
    is_8_adjacent(actor.pos, target.pos)
        && actor.life == LifeState::Alive
        && target.life == LifeState::Downed
        && actor.faction != target.faction
}

/// **Stabilize** a Downed ganger if the shared [`can_stabilize`] guard passes — the
/// E3.8 §9 verb (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md`
/// §"Downed → death … state machine").
///
/// When [`can_stabilize`] holds for `actor` over the `target` reads, this **sets**
/// the target's [`Stabilized`] flag to `Stabilized::new(true)` (mutating the passed
/// `&mut Stabilized` — the E3.7 flag is reused, this slice only sets it), so
/// [`crate::bleed::tick_bleed`] skips the ganger from the next round on. The target
/// **remains [`LifeState::Downed`]** — this verb never touches its [`LifeState`].
/// On success it returns `Some(`[`StabilizeTu`]`)`, the flat TU cost READ from
/// `tuning` (so the leaf is genuinely consulted); the cost is **not** debited from
/// any [`crate::ganger::Tu`] pool — that economy is **E4**.
///
/// When the guard is false (not adjacent, the actor not Alive, the target not Downed,
/// a cross-faction enemy, or the target already stabilized) this is a **no-op**:
/// it mutates nothing and returns `None` (predicate ⇔ act — the same guard the HUD
/// button reads). Pure, render-free mutation of a component reference — no pixel.
pub fn stabilize_downed(
    actor: &Actor,
    target: &DownedTarget,
    target_stabilized: &mut Stabilized,
    tuning: &CombatTuning,
) -> Option<StabilizeTu> {
    if !can_stabilize(actor, target) {
        return None;
    }
    // Set the E3.7 flag (reused, only set here) — the bleed clock halts; the ganger
    // stays Downed (this verb never writes LifeState).
    *target_stabilized = Stabilized::new(true);
    // READ the flat cost to wire the leaf — debiting a Tu pool is E4, not here.
    Some(tuning.stabilize_tu)
}

/// **Execute** a Downed ganger if the shared [`can_execute`] guard passes — the
/// E3.8 §9 verb (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md`
/// §"Downed → death … state machine").
///
/// When [`can_execute`] holds for `actor` over the `target` reads, this transitions
/// the target outright to [`LifeState::Dead`] (mutating the passed `&mut LifeState`)
/// — the enemy finisher. On success it returns `Some(`[`ExecuteTu`]`)`, the flat TU
/// cost READ from `tuning` (so the leaf is genuinely consulted); the cost is **not**
/// debited from any [`crate::ganger::Tu`] pool — that economy is **E4**.
///
/// When the guard is false (not adjacent, the actor not Alive, the target not Downed,
/// or a same-faction ally) this is a **no-op**: it mutates nothing and returns `None`
/// (predicate ⇔ act — the same guard the HUD button reads). Pure, render-free
/// mutation of a component reference — no pixel.
pub fn execute_downed(
    actor: &Actor,
    target: &DownedTarget,
    target_life: &mut LifeState,
    tuning: &CombatTuning,
) -> Option<ExecuteTu> {
    if !can_execute(actor, target) {
        return None;
    }
    // Finish the Downed ganger outright — the enemy executor.
    *target_life = LifeState::Dead;
    // READ the flat cost to wire the leaf — debiting a Tu pool is E4, not here.
    Some(tuning.execute_tu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metric::{Cell, CellLevel, Level};

    /// The ground storey (level 0) — the shared storey the same-level fixtures sit
    /// on (so the Moore-8 reach is exercised on one storey, with a deliberate
    /// off-storey case for the cross-level rejection).
    const GROUND: Level = Level::new(0);

    /// Build a [`Position`] from cell `(x, y)` on a storey — keeps the test bodies
    /// terse and pixel-free (all cubic-voxel cell/level coordinates).
    fn pos(x: i32, y: i32, level: Level) -> Position {
        Position::new(CellLevel::new(Cell::new(x, y), level))
    }

    /// Build an [`Actor`] bundle from its three gating reads.
    fn actor(pos: Position, life: LifeState, faction: u8) -> Actor {
        Actor {
            pos,
            life,
            faction: Faction::new(faction),
        }
    }

    /// Build a [`DownedTarget`] bundle; `stabilized` is the optional E3.7 flag.
    fn target(
        pos: Position,
        life: LifeState,
        faction: u8,
        stabilized: Option<bool>,
    ) -> DownedTarget {
        DownedTarget {
            pos,
            life,
            faction: Faction::new(faction),
            stabilized: stabilized.map(Stabilized::new),
        }
    }

    // --- is_8_adjacent: the same-level Moore-8 reach itself.

    /// All 8 surrounding same-level cells are adjacent; the center cell is not; a
    /// two-cell step is not.
    #[test]
    fn moore_8_neighbours_are_adjacent_self_and_far_are_not() {
        let center = pos(10, 10, GROUND);
        // The 8 ringing cells (dx, dy in {-1, 0, 1}, not both 0) are adjacent.
        for dx in -1..=1 {
            for dy in -1..=1 {
                let other = pos(10 + dx, 10 + dy, GROUND);
                let adjacent = is_8_adjacent(center, other);
                if dx == 0 && dy == 0 {
                    assert!(!adjacent, "the same cell is NOT 8-adjacent (excludes self)");
                } else {
                    assert!(
                        adjacent,
                        "({dx},{dy}) is one of the 8 surrounding cells — must be adjacent",
                    );
                }
            }
        }
        // A two-cell step (Chebyshev 2) is out of reach.
        assert!(
            !is_8_adjacent(center, pos(12, 10, GROUND)),
            "a two-cell step is NOT 8-adjacent",
        );
        assert!(
            !is_8_adjacent(center, pos(12, 12, GROUND)),
            "a two-cell diagonal step is NOT 8-adjacent",
        );
    }

    /// A neighbour on a DIFFERENT storey (same x/y, z off by one) is NOT adjacent —
    /// reach is the 8 same-level cells, not the 26 cross-level voxels.
    #[test]
    fn a_different_storey_is_not_adjacent() {
        let a = pos(5, 5, Level::new(2));
        // Directly above: same cell, one storey up.
        assert!(
            !is_8_adjacent(a, pos(5, 5, Level::new(3))),
            "the cell directly above is NOT 8-adjacent (same-level only)",
        );
        // A would-be Moore neighbour but a storey up — still not adjacent.
        assert!(
            !is_8_adjacent(a, pos(6, 5, Level::new(3))),
            "a Moore neighbour on another storey is NOT 8-adjacent",
        );
    }

    // === AC1 — can_stabilize is true ONLY for an Alive actor 8-adjacent to a
    // Downed, not-yet-Stabilized SAME-faction target; each guard, flipped alone,
    // independently makes it false. ===

    /// The canonical passing stabilize setup: an Alive ally one cell from a Downed,
    /// un-stabilized same-faction target.
    fn stabilize_pass() -> (Actor, DownedTarget) {
        let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
        let t = target(pos(11, 10, GROUND), LifeState::Downed, 1, None);
        (a, t)
    }

    #[test]
    fn can_stabilize_true_for_the_canonical_ally_setup() {
        let (a, t) = stabilize_pass();
        assert!(
            can_stabilize(&a, &t),
            "an Alive 8-adjacent same-faction actor stabilizes a Downed, un-stabilized target",
        );
    }

    #[test]
    fn can_stabilize_sweep_each_guard_flips_it_false() {
        // (1) NOT adjacent (two cells away).
        let (a, t) = stabilize_pass();
        let far = DownedTarget {
            pos: pos(13, 10, GROUND),
            ..t
        };
        assert!(!can_stabilize(&a, &far), "non-adjacent must fail");

        // (1b) NOT adjacent because a DIFFERENT level (same x/y, z+1).
        let other_storey = DownedTarget {
            pos: pos(11, 10, Level::new(1)),
            ..t
        };
        assert!(
            !can_stabilize(&a, &other_storey),
            "a same-x/y target one storey up is NOT adjacent — must fail",
        );

        // (2) actor not Alive (Downed actor).
        let downed_actor = Actor {
            life: LifeState::Downed,
            ..a
        };
        assert!(
            !can_stabilize(&downed_actor, &t),
            "a non-Alive (Downed) actor cannot stabilize",
        );
        let dead_actor = Actor {
            life: LifeState::Dead,
            ..a
        };
        assert!(
            !can_stabilize(&dead_actor, &t),
            "a Dead actor cannot stabilize",
        );

        // (3) target not Downed (Alive target).
        let alive_target = DownedTarget {
            life: LifeState::Alive,
            ..t
        };
        assert!(
            !can_stabilize(&a, &alive_target),
            "an Alive target is not a stabilize subject",
        );

        // (4) already stabilized (Some(true)).
        let already = DownedTarget {
            stabilized: Some(Stabilized::new(true)),
            ..t
        };
        assert!(
            !can_stabilize(&a, &already),
            "an already-stabilized target must fail (no re-dress)",
        );

        // (5) cross-faction (an ENEMY cannot stabilize).
        let enemy = DownedTarget {
            faction: Faction::new(2),
            ..t
        };
        assert!(
            !can_stabilize(&a, &enemy),
            "a cross-faction (enemy) actor cannot stabilize",
        );
    }

    /// A `Some(false)` flag is NOT "already stabilized" — present-and-false is
    /// not-yet-stabilized, so the guard still passes (distinguishes "has the
    /// component" from "is stabilized").
    #[test]
    fn can_stabilize_passes_with_stabilized_false_flag_present() {
        let (a, mut t) = stabilize_pass();
        t.stabilized = Some(Stabilized::new(false));
        assert!(
            can_stabilize(&a, &t),
            "Stabilized(false) present is not-yet-stabilized — the guard passes",
        );
    }

    // === AC2 — stabilize_downed SETS the flag and leaves LifeState::Downed
    // unchanged (NOT Alive, NOT Dead). ===

    #[test]
    fn stabilize_downed_sets_flag_and_keeps_downed() {
        let (a, t) = stabilize_pass();
        // Model the target's LifeState as the SAME `&mut` shape execute_downed
        // mutates, so this test would catch a stabilize verb that wrongly wrote it.
        let mut flag = Stabilized::new(false);
        let mut life = t.life;
        let tuning = CombatTuning::default();

        let acted = stabilize_downed(&a, &t, &mut flag, &tuning);
        // Apply the (non-)life-effect the same way the caller would: stabilize never
        // returns a LifeState change, so `life` stays whatever the target held.
        let _ = &mut life;

        assert!(acted.is_some(), "the canonical setup must act");
        assert_eq!(
            flag,
            Stabilized::new(true),
            "stabilize_downed must SET the Stabilized flag true",
        );
        // The verb takes NO &mut LifeState — the ganger stays Downed (NOT Alive, NOT
        // Dead): a stabilized ganger remains down, just no longer bleeding.
        assert_eq!(
            life,
            LifeState::Downed,
            "a stabilized ganger remains Downed (NOT Alive, NOT Dead)",
        );
    }

    // === AC3 — can_execute is true ONLY for an Alive actor 8-adjacent to a Downed
    // OPPOSING-faction target; parallel guard sweep. ===

    /// The canonical passing execute setup: an Alive enemy one cell from a Downed
    /// opposing-faction target.
    fn execute_pass() -> (Actor, DownedTarget) {
        let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
        let t = target(pos(11, 10, GROUND), LifeState::Downed, 2, None);
        (a, t)
    }

    #[test]
    fn can_execute_true_for_the_canonical_enemy_setup() {
        let (a, t) = execute_pass();
        assert!(
            can_execute(&a, &t),
            "an Alive 8-adjacent opposing-faction actor executes a Downed target",
        );
    }

    #[test]
    fn can_execute_sweep_each_guard_flips_it_false() {
        let (a, t) = execute_pass();

        // (1) NOT adjacent.
        let far = DownedTarget {
            pos: pos(13, 10, GROUND),
            ..t
        };
        assert!(!can_execute(&a, &far), "non-adjacent must fail");

        // (1b) NOT adjacent — different storey.
        let other_storey = DownedTarget {
            pos: pos(11, 10, Level::new(1)),
            ..t
        };
        assert!(
            !can_execute(&a, &other_storey),
            "a same-x/y target one storey up is NOT adjacent — must fail",
        );

        // (2) actor not Alive.
        let downed_actor = Actor {
            life: LifeState::Downed,
            ..a
        };
        assert!(
            !can_execute(&downed_actor, &t),
            "a Downed actor cannot execute"
        );

        // (3) target not Downed.
        let alive_target = DownedTarget {
            life: LifeState::Alive,
            ..t
        };
        assert!(
            !can_execute(&a, &alive_target),
            "an Alive target is not an execute subject",
        );

        // (4) same-faction (an ALLY cannot execute).
        let ally = DownedTarget {
            faction: Faction::new(1),
            ..t
        };
        assert!(
            !can_execute(&a, &ally),
            "a same-faction (ally) actor cannot execute",
        );
    }

    /// A stabilized Downed enemy can STILL be executed — the Stabilized flag does not
    /// gate execute (only stabilize is blocked by it).
    #[test]
    fn can_execute_ignores_stabilized_flag() {
        let (a, mut t) = execute_pass();
        t.stabilized = Some(Stabilized::new(true));
        assert!(
            can_execute(&a, &t),
            "a stabilized Downed enemy can still be executed (flag does not gate execute)",
        );
    }

    // === AC4 — execute_downed transitions the target to LifeState::Dead. ===

    #[test]
    fn execute_downed_kills_the_target() {
        let (a, t) = execute_pass();
        let mut life = LifeState::Downed;
        let tuning = CombatTuning::default();

        let acted = execute_downed(&a, &t, &mut life, &tuning);

        assert!(acted.is_some(), "the canonical setup must act");
        assert_eq!(
            life,
            LifeState::Dead,
            "execute_downed must transition the target outright to Dead",
        );
    }

    // === AC5 — the FACTION GATE: an enemy cannot stabilize; an ally cannot execute,
    // for both the predicate AND the act (no-op). ===

    #[test]
    fn faction_gate_enemy_cannot_stabilize() {
        // An Alive, 8-adjacent actor and a Downed target of a DIFFERENT faction.
        let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
        let t = target(pos(11, 10, GROUND), LifeState::Downed, 2, None);

        // Predicate: false.
        assert!(
            !can_stabilize(&a, &t),
            "an enemy (cross-faction) cannot stabilize — predicate false",
        );

        // Act: no-op (flag untouched, returns None).
        let mut flag = Stabilized::new(false);
        let tuning = CombatTuning::default();
        let acted = stabilize_downed(&a, &t, &mut flag, &tuning);
        assert!(
            acted.is_none(),
            "an enemy's stabilize_downed is a no-op (None)"
        );
        assert_eq!(
            flag,
            Stabilized::new(false),
            "an enemy's stabilize_downed must NOT set the flag",
        );
    }

    #[test]
    fn faction_gate_ally_cannot_execute() {
        // An Alive, 8-adjacent actor and a Downed target of the SAME faction.
        let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
        let t = target(pos(11, 10, GROUND), LifeState::Downed, 1, None);

        // Predicate: false.
        assert!(
            !can_execute(&a, &t),
            "an ally (same-faction) cannot execute — predicate false",
        );

        // Act: no-op (life untouched, returns None).
        let mut life = LifeState::Downed;
        let tuning = CombatTuning::default();
        let acted = execute_downed(&a, &t, &mut life, &tuning);
        assert!(
            acted.is_none(),
            "an ally's execute_downed is a no-op (None)"
        );
        assert_eq!(
            life,
            LifeState::Downed,
            "an ally's execute_downed must NOT kill the target",
        );
    }

    // === AC6 — PREDICATE ⇔ ACT: each act is a no-op EXACTLY when its predicate is
    // false (both directions). ===

    #[test]
    fn stabilize_act_iff_predicate_both_directions() {
        let tuning = CombatTuning::default();

        // Predicate TRUE → act fires (sets flag, returns Some).
        let (a, t) = stabilize_pass();
        let mut flag = Stabilized::new(false);
        assert!(can_stabilize(&a, &t));
        let acted = stabilize_downed(&a, &t, &mut flag, &tuning);
        assert!(acted.is_some(), "predicate true ⇒ act fires");
        assert_eq!(flag, Stabilized::new(true), "predicate true ⇒ flag set");

        // Predicate FALSE (here: non-adjacent) → act is a no-op (None, flag intact).
        let far = DownedTarget {
            pos: pos(20, 20, GROUND),
            ..t
        };
        let mut flag2 = Stabilized::new(false);
        assert!(!can_stabilize(&a, &far));
        let acted2 = stabilize_downed(&a, &far, &mut flag2, &tuning);
        assert!(acted2.is_none(), "predicate false ⇒ no-op (None)");
        assert_eq!(
            flag2,
            Stabilized::new(false),
            "predicate false ⇒ flag untouched",
        );
    }

    #[test]
    fn execute_act_iff_predicate_both_directions() {
        let tuning = CombatTuning::default();

        // Predicate TRUE → act fires (kills, returns Some).
        let (a, t) = execute_pass();
        let mut life = LifeState::Downed;
        assert!(can_execute(&a, &t));
        let acted = execute_downed(&a, &t, &mut life, &tuning);
        assert!(acted.is_some(), "predicate true ⇒ act fires");
        assert_eq!(life, LifeState::Dead, "predicate true ⇒ target Dead");

        // Predicate FALSE (here: non-adjacent) → act is a no-op (None, life intact).
        let far = DownedTarget {
            pos: pos(20, 20, GROUND),
            ..t
        };
        let mut life2 = LifeState::Downed;
        assert!(!can_execute(&a, &far));
        let acted2 = execute_downed(&a, &far, &mut life2, &tuning);
        assert!(acted2.is_none(), "predicate false ⇒ no-op (None)");
        assert_eq!(
            life2,
            LifeState::Downed,
            "predicate false ⇒ target unchanged (still Downed)",
        );
    }

    // === AC7 — the verbs READ the flat TU cost from tuning (the leaf is genuinely
    // consulted), without running the TU economy (E4). The shipped-RON parse pin
    // lives in tuning::tests. ===

    /// On success each act returns the cost it READ from tuning — equal to that
    /// tuning leaf (a RELATION to the value, never a pinned magnitude). This proves
    /// the leaf is non-vacuously read.
    #[test]
    fn acts_return_the_tu_cost_read_from_tuning() {
        // A non-default tuning so the returned cost provably came FROM tuning, not a
        // hardcoded constant (the values themselves stay arbitrary, not pinned).
        let tuning = CombatTuning {
            stabilize_tu: StabilizeTu::new(9),
            execute_tu: ExecuteTu::new(13),
            ..CombatTuning::default()
        };

        let (a, t) = stabilize_pass();
        let mut flag = Stabilized::new(false);
        let stab_cost = stabilize_downed(&a, &t, &mut flag, &tuning);
        assert_eq!(
            stab_cost,
            Some(tuning.stabilize_tu),
            "stabilize_downed must return the stabilize_tu READ from tuning",
        );

        let (ea, et) = execute_pass();
        let mut life = LifeState::Downed;
        let exec_cost = execute_downed(&ea, &et, &mut life, &tuning);
        assert_eq!(
            exec_cost,
            Some(tuning.execute_tu),
            "execute_downed must return the execute_tu READ from tuning",
        );
    }
}
