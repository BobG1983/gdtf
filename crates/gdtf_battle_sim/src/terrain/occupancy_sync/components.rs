//! The maintenance-layer bookkeeping types: the per-entity [`PrevSlot`] memory and
//! the buffered [`CoverDestroyed`] / [`SlabDestroyed`] / [`GroundAccrued`] messages.

use bevy::prelude::{Component, Message};

use crate::{
    metric::{Cell, CellLevel},
    surface::GroundDamage,
};

/// The `(cell, level)` slot an entity was **last synced into** the occupancy grid
/// at — the per-entity bookkeeping
/// [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) uses to clear
/// the OLD slot on a move.
///
/// `Changed<`[`Position`](crate::ganger::Position)`>` only ever yields the entity's
/// *new* [`Position`](crate::ganger::Position), so the move system cannot tell where
/// the entity *was* without remembering it. This component is that memory: the sync
/// systems WRITE it (the maintenance layer owns it — it is not authored ganger
/// state), and read it back to know which slot's occupant marker to clear.
///
/// **GTW-391 dual-cell stair occupancy.** A non-prone ganger standing on a stair tile
/// occupies TWO grid cells — the lower `(cell, level)` where its `Position` lives, and
/// the upper `(cell, level+1)` whose `Low` band represents the ganger's body protruding
/// into the next storey. [`PrevSlot`] records BOTH so that every teardown path (move,
/// prone re-pose, death) can replay exactly what was written — never re-deriving the
/// upper cell at teardown time. The [`upper`](PrevSlot::upper) field is `None` when
/// there is no upper presence (non-stair, prone, top storey, or upper cell already
/// occupied by another entity — the single-occupant-slot constraint; see
/// [`OccupancyGrid::register_stair_presence`](crate::occupancy::OccupancyGrid::register_stair_presence)).
///
/// The inner fields are PRIVATE (no-bare-types rule 5): always construct via
/// [`PrevSlot::new`] or [`PrevSlot::with_upper`], read through [`slot`](PrevSlot::slot)
/// / [`upper`](PrevSlot::upper). No public `Deref` — callers compare/read through the
/// named accessors.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrevSlot {
    /// The lower `(cell, level)` this entity's [`Position`] occupies — always present.
    lower: CellLevel,
    /// The upper `(cell, level+1)` this entity ALSO occupies when it is a non-prone
    /// stair occupant, or `None` (non-stair, prone, top storey, or upper cell taken).
    /// Teardown replays this field verbatim — it never re-derives the upper cell.
    upper: Option<CellLevel>,
}

impl PrevSlot {
    /// Build a previously-synced-slot marker for `lower` with no upper stair presence.
    ///
    /// Signature-identical to the pre-GTW-391 `PrevSlot::new(slot)` so every existing
    /// call site (the move system, the existing round-trip test) compiles unchanged.
    #[must_use]
    pub const fn new(lower: CellLevel) -> Self {
        Self { lower, upper: None }
    }

    /// Build a previously-synced-slot marker recording BOTH `lower` AND the upper stair
    /// cell `upper` — the dual-cell form for a non-prone stair occupant.
    ///
    /// The caller obtains `upper` as the return value of
    /// [`OccupancyGrid::register_stair_presence`](crate::occupancy::OccupancyGrid::register_stair_presence),
    /// which applies the occupancy guard (so `upper` is always a real written cell).
    #[must_use]
    pub const fn with_upper(lower: CellLevel, upper: CellLevel) -> Self {
        Self {
            lower,
            upper: Some(upper),
        }
    }

    /// The lower `(cell, level)` slot this marker records — the slot to clear when the
    /// entity moves on or goes down (the entity's authoritative
    /// [`Position`](crate::ganger::Position) cell).
    #[must_use]
    pub const fn slot(self) -> CellLevel {
        self.lower
    }

    /// The upper `(cell, level+1)` this entity ALSO occupies (stair, non-prone, upper
    /// cell free at registration time), or `None`. Teardown reads this and replays it
    /// verbatim — never re-derives the upper cell at teardown time.
    #[must_use]
    pub const fn upper(self) -> Option<CellLevel> {
        self.upper
    }
}

/// A piece of cover was **destroyed** at a `(cell, level)` — the buffered message
/// [`sync_destroyed_cover`](crate::occupancy_sync::sync_destroyed_cover) folds into
/// the grid's destroyed-cover set.
///
/// The sim's destroyed-cover signal is `CoverDestroyed { cell, level }` (a
/// model-authoritative fact the view mirrors; ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`); the GTW-154
/// [`crate::cover::CoverEvent::Destroyed`]
/// depletion result is what becomes this message (a deplete→message bridge is a
/// later slice — this slice consumes the message). Carries a [`CellLevel`]
/// (no-bare-types; never a numeric id). It is a **buffered message**, NOT the
/// observer `Event` API: Bevy 0.18 renamed buffered `Event`/`EventReader` to
/// `Message`/`MessageReader` (`bevy-traps.md` #4 — the ticket's "`EventReader`"
/// is pre-0.18 terminology, identical semantics), so it `#[derive(Message)]` and
/// is read with [`MessageReader`](bevy::prelude::MessageReader).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDestroyed {
    /// The `(cell, level)` whose cover was destroyed — added to the grid's
    /// append-only destroyed-cover set.
    pub at: CellLevel,
}

impl CoverDestroyed {
    /// Build a cover-destroyed message for the `(cell, level)` that was smashed.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self { at }
    }
}

/// A floor/roof **slab** was **destroyed** at a `(cell, level)` — the buffered message
/// [`sync_destroyed_slab`](crate::occupancy_sync::sync_destroyed_slab) folds into the
/// [`SurfaceGrid`](crate::surface::SurfaceGrid) via
/// [`destroy_slab`](crate::surface::SurfaceGrid::destroy_slab), and which
/// [`should_recompute_visibility`](crate::visibility::should_recompute_visibility)
/// drains to re-reveal the opened sightline (GTW-365; the slab mirror of
/// [`CoverDestroyed`]).
///
/// The sim's destroyed-slab signal is `SlabDestroyed { cell, level }` (a
/// model-authoritative fact the view mirrors — ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`; the presenter `SlabDestroyed` reaction is
/// GTW-367). The [`SlabEvent::Destroyed`](crate::slab::SlabEvent::Destroyed) depletion
/// result is what becomes this message via the fire→deplete→message bridge in
/// `dispatch_fire`. Carries a [`CellLevel`] (no-bare-types; never a numeric id). A
/// **buffered message** (Bevy 0.18 renamed buffered `Event`/`EventReader` to
/// `Message`/`MessageReader` — `bevy-traps.md` #4), so it `#[derive(Message)]` and is
/// read with [`MessageReader`](bevy::prelude::MessageReader).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDestroyed {
    /// The `(cell, level)` whose slab was destroyed — set
    /// [`SlabState::Destroyed`](crate::surface::SlabState) on the surface grid
    /// (idempotent / permanent).
    pub at: CellLevel,
}

impl SlabDestroyed {
    /// Build a slab-destroyed message for the `(cell, level)` whose slab was smashed.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self { at }
    }
}

/// The ground at a [`Cell`] **took damage** — the buffered message
/// [`sync_accrued_ground`](crate::occupancy_sync::sync_accrued_ground) folds into the
/// [`SurfaceGrid`](crate::surface::SurfaceGrid)'s per-cell ground accumulator via
/// [`accrue_ground_damage`](crate::surface::SurfaceGrid::accrue_ground_damage) (GTW-366;
/// the ground-accrual mirror of [`SlabDestroyed`]).
///
/// A round that exits the bottom of the voxel column strikes the ground — **damaged,
/// never destroyed** (`docs/combat/resolution.md` §3.2; user-ruled 2026-06-22). The
/// E3.9 fold records the struck [`Cell`] and the round's [`GroundDamage`] on
/// [`GroundAccrual`](crate::resolve_and_apply::GroundAccrual), and
/// the fire path's [`dispatch_fire`](crate::acts::dispatch_fire) bridges it into THIS
/// buffered message — the same fire→message→sync shape the cover / slab destruction
/// bridges use, except the consumer ACCRUES (monotonically, never lowers) onto the
/// [`SurfaceGrid`](crate::surface::SurfaceGrid) the fold does not hold, rather than
/// destroying anything. Purely cosmetic bookkeeping for a future crater-FX reaction (the
/// crater render is out of scope — GTW-366 C5).
///
/// Carries the [`Cell`] hit and the [`GroundDamage`] amount (both no-bare-types; the
/// cell is the §3 ground-plane cell, the amount the round's `weapon_damage`). A
/// **buffered message** (Bevy 0.18 renamed buffered `Event`/`EventReader` to
/// `Message`/`MessageReader` — `bevy-traps.md` #4), so it `#[derive(Message)]` and is
/// read with [`MessageReader`](bevy::prelude::MessageReader).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroundAccrued {
    /// The ground-plane [`Cell`] the round struck — the accumulator key
    /// [`accrue_ground_damage`](crate::surface::SurfaceGrid::accrue_ground_damage) adds to.
    pub cell:   Cell,
    /// The [`GroundDamage`] the round dealt — the round's `weapon_damage`, accrued
    /// (saturating, monotonic) onto the cell's running total.
    pub amount: GroundDamage,
}

impl GroundAccrued {
    /// Build a ground-accrued message for the `cell` the round struck and the
    /// `amount` of [`GroundDamage`] it dealt (the round's `weapon_damage`).
    #[must_use]
    pub const fn new(cell: Cell, amount: GroundDamage) -> Self {
        Self { cell, amount }
    }
}
