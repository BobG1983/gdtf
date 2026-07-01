//! The pure **shove-displacement verb** (GTW-525 C1) — [`resolve_shove`], which resolves the
//! one-cell knock-back away from the attacker and its supported / unsupported / blocked
//! branch, REUSING [`resolve_drop`](crate::falls::resolve_drop) for the unsupported=>fall arm.
//!
//! Render-free, `SurfaceGrid` + `OccupancyGrid`-only, no ECS, **no RNG**: given the attacker's
//! and target's `(cell, level)` and the two persistent grids, it computes the destination cell
//! one step directly AWAY from the attacker (along the attacker->target facing) and resolves
//! it into a [`ShoveOutcome`]:
//!
//! - **BLOCKED** (the destination is off-direction, occupied by ANOTHER ganger, or a
//!   solid/blocked terrain cell) => [`ShoveOutcome::Blocked`] (a no-op — never shove into a
//!   solid);
//! - **SUPPORTED** (the destination is on the ground, level `0`, OR its slab reads
//!   [`SlabState::Present`](crate::surface::SlabState)) => [`ShoveOutcome::Moved`] one cell,
//!   no fall;
//! - **UNSUPPORTED** (level `≥ 1` AND the destination slab is NOT `Present`) =>
//!   [`ShoveOutcome::Fell`]: the target is moved to the destination and FALLS, its landing
//!   resolved by the SHARED [`resolve_drop`](crate::falls::resolve_drop) (the GTW-523 drop
//!   scan — NO reimplemented fall geometry).
//!
//! The shove itself draws NO RNG (it is not a contested roll — the action always shoves an
//! adjacent target; the tag always shoves on a connecting hit). Only the resulting FALL draws
//! RNG, via GTW-523's severity / injury streams — applied by the caller through the shared
//! fall-damage fork, NOT here.

use crate::{
    falls::{DropLanding, resolve_drop},
    ganger::{Direction, Position},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
};

/// The resolved outcome of one shove-displacement (GTW-525 C1) — what became of the target
/// pushed one cell away from the attacker.
///
/// A `Copy` value object of named domain types. The caller (the deliberate act OR a weapon-tag
/// auto-shove hook) reads this to decide what to apply: nothing on [`Blocked`](Self::Blocked),
/// a `Position` rewrite on [`Moved`](Self::Moved), and a `Position` rewrite + the shared
/// GTW-523 fall-damage fold + a `FallOccurred` signal on [`Fell`](Self::Fell). The shove verb
/// resolves the geometry; the caller owns the mutation + the RNG-bearing fall damage — keeping
/// the verb pure and RNG-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShoveOutcome {
    /// The shove is a NO-OP — the destination is not a legal cell to push into (a co-located
    /// target with no direction, a ganger-occupied cell, or a solid / blocked terrain cell).
    /// Never shove into a solid. The caller applies nothing.
    Blocked,
    /// The target is pushed one cell to `dest` and stays STANDING there — the destination has a
    /// supporting floor (the ground `level 0`, or a `Present` slab). No fall. The caller
    /// rewrites the target's [`Position`] to `dest`.
    Moved {
        /// The destination `(cell, level)` the target is moved to (same storey as the start —
        /// a shove is a lateral push).
        dest: CellLevel,
    },
    /// The target is pushed one cell to `dest` and FALLS — the destination has no supporting
    /// floor at its storey, so it drops to `landing.landing` (the GTW-523 drop scan). The
    /// caller rewrites the target's [`Position`] to `(dest.cell, landing.landing)` and routes
    /// the fall through the SHARED GTW-523 fall-damage fork + emits `FallOccurred`.
    Fell {
        /// The destination cell the target was pushed onto (its `(x, y)`; the storey it fell
        /// FROM is `dest.z`, the same storey as the start).
        dest:    CellLevel,
        /// The resolved landing — the storey the target lands ON and the storeys it fell — from
        /// the SHARED [`resolve_drop`](crate::falls::resolve_drop) (never a reimplemented scan).
        landing: DropLanding,
    },
}

/// The ground-plane [`Cell`] of a [`Position`] — its `(x, y)` (the `apply_falls` /
/// `dispatch_melee` `ganger_cell` precedent).
fn position_cell(position: Position) -> Cell {
    let key = **position;
    Cell::new(key.x, key.y)
}

/// The storey [`Level`] of a [`Position`] — its `z` storey index (the `apply_falls`
/// `faller_level` precedent).
fn position_level(position: Position) -> Level {
    let key = **position;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "z is a storey index in 0..MAX_LEVELS (8) by construction, so the i32 -> u8 \
                  narrowing cannot truncate or sign-flip (the apply_falls faller_level precedent)"
    )]
    let storey = key.z as u8;
    Level::new(storey)
}

/// The cell one step in `direction` from `cell` — the destination a shove pushes the target
/// onto (the 8-way [`Direction`] step in the ground plane).
///
/// Maps each of the eight [`Direction`] variants to its DISCRETE `(dx, dy)` cell offset (the
/// `−Y = North`, `+X = East` grid convention `Direction` documents), so a cardinal moves one
/// cell on one axis and a diagonal one cell on each of two — the single-cell knock-back the
/// shove specifies. Integer offsets (NOT the float
/// [`forward_step`](Direction::forward_step) unit vector, whose `f32::signum` of a `0.0`
/// component is `+1.0`, never `0.0` — a cardinal would wrongly gain a diagonal component).
/// Pure integer arithmetic over the cubic-voxel metric; no pixel.
fn step_cell(cell: Cell, direction: Direction) -> Cell {
    let (dx, dy) = match direction {
        Direction::North => (0, -1),
        Direction::NorthEast => (1, -1),
        Direction::East => (1, 0),
        Direction::SouthEast => (1, 1),
        Direction::South => (0, 1),
        Direction::SouthWest => (-1, 1),
        Direction::West => (-1, 0),
        Direction::NorthWest => (-1, -1),
    };
    Cell::new(cell.x + dx, cell.y + dy)
}

/// Resolve the one-cell **shove displacement** away from the attacker (GTW-525 C1) — the pure
/// §Shove geometry verb.
///
/// Computes the destination as ONE cell from `target` directly AWAY from `attacker` (the
/// attacker->target [`Direction`], stepped one cell on the SAME storey), then resolves it:
///
/// 1. **Direction.** [`Direction::from_cells`]`(attacker_cell, target_cell)` — the way the push
///    points. Co-located attacker + target (no direction) => [`ShoveOutcome::Blocked`] (a
///    defensive guard; a real shove target is 8-adjacent, never co-located).
/// 2. **Blocked.** If the destination `(dest, level)` is occupied by ANOTHER ganger
///    ([`OccupancyGrid::occupant`] is `Some` and NOT the target itself) OR its terrain
///    BLOCKS ([`OccupancyGrid::is_blocked`] — a wall / standing cover) => [`ShoveOutcome::Blocked`]
///    (never shove into a solid).
/// 3. **Supported / unsupported.** The destination is SUPPORTED iff `level == 0` (the ground
///    always supports) OR `slab_state((dest, level)) == Present` (an intact floor) — matching
///    the [`resolve_drop`](crate::falls::resolve_drop) support predicate exactly. Supported =>
///    [`ShoveOutcome::Moved`]; unsupported => the target falls, its landing from
///    [`resolve_drop`]`(dest, level, surface)` => [`ShoveOutcome::Fell`].
///
/// Deterministic and pure: reads the two grids, mutates nothing, and draws NO RNG (the shove
/// is not a contested roll). A `level == 0` unsupported case cannot arise (level 0 always
/// supports), and [`resolve_drop`] returns `None` only for `start == 0` (which the support
/// check already routed to `Moved`), so the `Fell` arm's `resolve_drop` is always `Some`; a
/// defensive `None` degrades to `Moved` (no fall) rather than panicking. `target_entity` keys
/// the self-occupancy exclusion (the target still occupies its OWN start cell, not the dest,
/// but a defensive check keeps a self-blocked no-op impossible).
#[must_use]
pub fn resolve_shove(
    attacker: Position,
    target: Position,
    target_entity: bevy::prelude::Entity,
    surface: &SurfaceGrid,
    occupancy: &OccupancyGrid,
) -> ShoveOutcome {
    let attacker_cell = position_cell(attacker);
    let target_cell = position_cell(target);
    let level = position_level(target);

    // (1) The push direction — attacker -> target (away from the attacker). A co-located pair
    // has no direction (defensive: a real target is 8-adjacent) => no shove.
    let Some(direction) = Direction::from_cells(attacker_cell, target_cell) else {
        return ShoveOutcome::Blocked;
    };
    let dest_cell = step_cell(target_cell, direction);
    let dest = CellLevel::new(dest_cell, level);

    // (2) Blocked — never shove into a solid. A ganger occupant OTHER than the target itself,
    // or a blocking terrain cell (wall / standing cover), rejects the shove (no-op).
    if let Some(occupant) = occupancy.occupant(&dest)
        && occupant != target_entity
    {
        return ShoveOutcome::Blocked;
    }
    if occupancy.is_blocked(&dest) {
        return ShoveOutcome::Blocked;
    }

    // (3) Supported (ground level 0, or an intact Present slab) => a lateral move, no fall.
    // Unsupported => the target falls; the landing is the SHARED GTW-523 drop scan.
    let supported = *level == 0 || surface.slab_state(&dest) == crate::surface::SlabState::Present;
    if supported {
        return ShoveOutcome::Moved { dest };
    }
    match resolve_drop(dest_cell, level, surface) {
        Some(landing) => ShoveOutcome::Fell { dest, landing },
        // Defensive: resolve_drop returns None only for start == 0, which the supported check
        // above already routed to Moved — so this arm is unreachable for level >= 1. Degrade to
        // a plain move (no fall) rather than panicking (fail-closed).
        None => ShoveOutcome::Moved { dest },
    }
}
