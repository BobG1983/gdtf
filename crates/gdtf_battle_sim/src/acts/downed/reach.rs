//! The shared 8-adjacency reach + the actor/target read-bundles both from-Downed
//! verbs gate on.

use bevy::prelude::Deref;

use crate::ganger::{Faction, LifeState, Position, Stabilized};

/// Whether two gangers occupy the same-level Moore-8 neighbourhood — the
/// [`is_8_adjacent`] reach verdict (`docs/combat/resolution.md` §9).
///
/// The tactical-reach answer both from-Downed verbs gate on: `true` means the two
/// cells are 8-adjacent (a stabilize / execute is within reach), `false` means out of
/// reach. A distinct domain predicate, not a bare `bool`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adjacent8(bool);

impl Adjacent8 {
    /// Build the 8-adjacency verdict from the computed reach test.
    #[must_use]
    pub const fn new(adjacent: bool) -> Self {
        Self(adjacent)
    }
}

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
pub fn is_8_adjacent(a: Position, b: Position) -> Adjacent8 {
    // The (cell, level) keys: x/y are cells, z is the storey index (metric.rs).
    let pa = **a;
    let pb = **b;
    // Same storey: a ganger above or below is NOT in the 8 surrounding cells.
    if pa.z != pb.z {
        return Adjacent8::new(false);
    }
    let dx = (pa.x - pb.x).abs();
    let dy = (pa.y - pb.y).abs();
    // Chebyshev distance exactly 1: the 8 cells ringing the actor, excluding itself
    // (dx == 0 && dy == 0 → the same cell, not adjacent).
    Adjacent8::new(dx <= 1 && dy <= 1 && (dx != 0 || dy != 0))
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
    /// not-yet-stabilized. Unused by [`can_execute`](crate::acts::downed::can_execute)
    /// (an executable ganger may be stabilized or not).
    pub stabilized: Option<Stabilized>,
}
