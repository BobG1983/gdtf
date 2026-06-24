//! The maintenance-layer bookkeeping types: the per-entity [`PrevSlot`] memory and
//! the buffered [`CoverDestroyed`] message.

use bevy::prelude::{Component, Message};

use crate::metric::CellLevel;

/// The `(cell, level)` slot an entity was **last synced into** the occupancy grid
/// at — the per-entity bookkeeping
/// [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) uses to clear
/// the OLD slot on a move.
///
/// `Changed<`[`Position`](crate::ganger::Position)`>` only ever yields the entity's
/// *new* [`Position`](crate::ganger::Position), so the move system cannot tell where
/// the entity *was* without remembering it. This component is that memory: the sync
/// systems WRITE it (the maintenance layer owns it — it is not authored ganger
/// state), and read it back to know which slot's occupant marker to clear. A named
/// newtype over [`CellLevel`] (no-bare-types: a tracked slot is a domain value, not
/// a bare key); private inner + a [`new`](PrevSlot::new) constructor, no public
/// `Deref` since callers only ever compare/read the whole slot through
/// [`slot`](PrevSlot::slot).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrevSlot(CellLevel);

impl PrevSlot {
    /// Build a previously-synced-slot marker for the `(cell, level)` an entity was
    /// last synced into.
    #[must_use]
    pub const fn new(slot: CellLevel) -> Self {
        Self(slot)
    }

    /// The `(cell, level)` slot this marker records — the slot to clear when the
    /// entity moves on or goes down.
    #[must_use]
    pub const fn slot(self) -> CellLevel {
        self.0
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
