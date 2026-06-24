//! Change-driven occupancy maintenance — the E1.7 sync slice (GTW-157).
//!
//! GTW-156 ([`crate::occupancy`]) shipped the
//! [`OccupancyGrid`](crate::occupancy::OccupancyGrid) resource with a
//! `build_from_occupancy_input` constructor that pours an occupancy input into a
//! fresh grid. That constructor is the **setup-time** pour, NOT the per-frame
//! maintenance path: per the **change-driven grid maintenance** thesis (the
//! GTW-6 / GTW-12 architectural ruling; the change-driven sim↔app seam recorded in
//! ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`), the live
//! battle keeps the grid current by reacting to the **changes** — a ganger moved,
//! a ganger went down, a piece of cover was smashed — and editing the grid IN
//! PLACE, never by re-running a full-grid rebuild per shot.
//!
//! This module is that maintenance layer. It is five focused Bevy systems — three
//! sharing `ResMut<`[`OccupancyGrid`](crate::occupancy::OccupancyGrid)`>`, plus the
//! slab-surface one (GTW-365) and the ground-accrual one (GTW-366) on
//! `ResMut<`[`SurfaceGrid`](crate::surface::SurfaceGrid)`>` — each driven by a SINGLE
//! trigger:
//!
//! 1. [`sync_moved_gangers`] reacts to `Changed<`[`Position`](crate::ganger::Position)`>`
//!    (Bevy change detection): it clears the entity's OLD occupancy slot and marks
//!    its NEW one. Because `Changed<Position>` only ever yields the *new*
//!    [`Position`](crate::ganger::Position), the system tracks each entity's
//!    previously-synced slot in a [`PrevSlot`] component it writes itself — that is
//!    how it knows which old slot to clear (the ticket-sanctioned "track the
//!    previous slot" approach).
//! 2. [`sync_dead_gangers`] reacts to `Changed<`[`LifeState`](crate::ganger::LifeState)`>`
//!    filtered to a non-[`LifeState::Alive`](crate::ganger::LifeState::Alive) state
//!    (Downed / Dead): it clears that entity's occupant marker from the slot it last
//!    synced to.
//! 3. [`sync_destroyed_cover`] reads the buffered [`CoverDestroyed`] **message**
//!    (Bevy 0.18 renamed buffered events to messages — `bevy-traps.md` #4) and
//!    folds each one's [`CellLevel`](crate::metric::CellLevel) into the grid's
//!    append-only destroyed-cover set via
//!    [`OccupancyGrid::mark_cover_destroyed`](crate::occupancy::OccupancyGrid::mark_cover_destroyed).
//! 4. [`sync_destroyed_slab`] (GTW-365) reads the buffered [`SlabDestroyed`] message
//!    and folds each one's [`CellLevel`](crate::metric::CellLevel) into the
//!    [`SurfaceGrid`](crate::surface::SurfaceGrid) via
//!    [`SurfaceGrid::destroy_slab`](crate::surface::SurfaceGrid::destroy_slab)
//!    (idempotent / permanent) — the slab mirror of `sync_destroyed_cover`, on the
//!    persistent surface grid rather than the per-shot occupancy grid.
//! 5. [`sync_accrued_ground`] (GTW-366) reads the buffered [`GroundAccrued`] message
//!    and folds each one's [`Cell`](crate::metric::Cell) +
//!    [`GroundDamage`](crate::surface::GroundDamage) into the
//!    [`SurfaceGrid`](crate::surface::SurfaceGrid) via
//!    [`SurfaceGrid::accrue_ground_damage`](crate::surface::SurfaceGrid::accrue_ground_damage)
//!    (monotonic — the accumulator only grows) — the ground-accrual mirror of
//!    `sync_destroyed_slab`, purely cosmetic (the ground is damaged, never destroyed).
//!
//! **Change detection / the message reader is the ONLY trigger.** There is no
//! polling, no per-shot full-grid scan, and NOTHING here calls
//! [`OccupancyGrid::build_from_occupancy_input`](crate::occupancy::OccupancyGrid::build_from_occupancy_input)
//! — a rebuild-per-shot is the exact anti-pattern this slice exists to replace.
//!
//! [`OccupancyMaintenancePlugin`] is the wiring unit: it registers the
//! [`CoverDestroyed`] + [`SlabDestroyed`] + [`GroundAccrued`] message buffers and adds the
//! five systems to [`Update`](bevy::prelude::Update). The three `OccupancyGrid` systems run
//! in an EXPLICIT [`chain`](bevy::prelude::IntoScheduleConfigs::chain) order
//! (`bevy-traps.md` #3 — three systems sharing one `ResMut` must be ordered
//! deterministically); [`sync_destroyed_slab`] writes a DIFFERENT resource
//! (`ResMut<SurfaceGrid>`), so it needs no ordering against that chain — but it is
//! ordered alongside it in the same set so a destroyed slab's surface edit lands
//! before the squad fog recomputes (the recompute is ordered `.after` the set).
//! [`sync_accrued_ground`] ALSO writes `ResMut<SurfaceGrid>`, so it is chained
//! `.after(sync_destroyed_slab)` (two systems on one resource — the same explicit-order
//! discipline); its accrual is cosmetic, so no recompute follows it. The
//! production app adds this plugin when the sim is wired into the runtime (E1.8 / E5);
//! that app-wiring is out of scope here, so the systems are exercised by this module's
//! headless tests.

mod components;
mod plugin;
mod systems;

#[cfg(test)]
mod test;

pub use components::{CoverDestroyed, GroundAccrued, PrevSlot, SlabDestroyed};
pub use plugin::{OccupancyMaintenancePlugin, SimSystems};
pub use systems::{
    sync_accrued_ground, sync_dead_gangers, sync_destroyed_cover, sync_destroyed_slab,
    sync_moved_gangers,
};
