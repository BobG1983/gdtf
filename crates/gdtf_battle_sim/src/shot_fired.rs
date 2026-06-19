//! The [`ShotFired`] signal — the per-round fire-trajectory message the presenter
//! reads to draw the GTW-290 muzzle / tracer / impact FX.
//!
//! The shot's geometry is ALREADY computed by the coarse pipeline
//! ([`ShotOutcome`](crate::resolve_coarse::ShotOutcome) in
//! [`resolve_coarse`](crate::resolve_coarse::resolve_coarse)) and folded into the
//! frozen volley [`fire`](crate::fire::fire) returns; this message simply EXPOSES that
//! already-resolved geometry so it is not dropped. It carries NO new fire-result logic
//! and recomputes nothing — every field is copied straight off the round's
//! [`ShotOutcome`].
//!
//! A buffered Bevy **message** (`#[derive(Message)]`), mirroring
//! [`crate::bleed::Bleeding`] / [`crate::occupancy_sync::CoverDestroyed`] /
//! [`crate::armor_wear::ArmorBroken`] — NOT the observer `Event` API
//! (`bevy-traps.md` #4), so it is written with [`bevy::prelude::MessageWriter`] and read
//! with [`bevy::prelude::MessageReader`]. One [`ShotFired`] is emitted per ROUND resolved
//! in [`dispatch_fire`](crate::acts::dispatch_fire) (a burst / full-auto volley fires
//! multiple rounds → multiple [`ShotFired`], so the presenter draws a tracer per round).
//!
//! The sim stays RENDER-FREE: every position rides in **sim units** — a cubic-voxel
//! [`SimPos`] muzzle, a unit-[`Vec3`](bevy::math::Vec3) [`ShotDir`] trajectory, and a
//! discrete `(`[`Cell`]`, `[`Level`]`)` impact — never a pixel, never a sprite, never a
//! colour. The presenter maps those sim units to screen space; the sim never reads the
//! presenter (the one-way `input -> presenter -> sim` edge, ADR-0001).

use bevy::prelude::{Entity, Message};

use crate::{
    metric::{Cell, Level, SimPos},
    resolve_coarse::{ShotKind, ShotOutcome},
    sample_cone::ShotDir,
};

/// One **round was fired** — the per-round trajectory geometry the presenter draws the
/// GTW-290 muzzle / tracer / impact FX from.
///
/// Emitted once per round resolved in [`dispatch_fire`](crate::acts::dispatch_fire),
/// AFTER the volley resolves (so the geometry is final), sourced field-by-field from the
/// round's already-computed [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) — it adds
/// no fire-result logic and recomputes nothing.
///
/// Every position is a **sim unit**, never a pixel: [`muzzle`](ShotFired::muzzle) is the
/// 3D fire origin ([`SimPos`]), [`trajectory`](ShotFired::trajectory) is the round's unit
/// direction ([`ShotDir`]), and [`impact_cell`](ShotFired::impact_cell) /
/// [`impact_level`](ShotFired::impact_level) are the discrete impact `(cell, level)`. The
/// [`kind`](ShotFired::kind) is the [`ShotKind`] the shot struck (ganger / cover / slab /
/// ground / miss) — a clean miss still carries muzzle + trajectory + the cell the round
/// left through, so the presenter draws a tracer terminating at the impact cell.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`). The
/// [`shooter`](ShotFired::shooter) is a Bevy [`Entity`] handle — framework plumbing, the
/// only bare type the no-bare-types rule permits in a payload. Derives [`PartialEq`] (NOT
/// [`Eq`]: the [`SimPos`] / [`ShotDir`] hold `f32`, so equality is bit-wise, the
/// seeded-replay property).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ShotFired {
    /// The firing entity (the armed shooter the round left).
    pub shooter:      Entity,
    /// The 3D muzzle point the round was fired from, in sim units (a cubic-voxel
    /// [`SimPos`]) — the tracer's origin + the muzzle-flash position.
    pub muzzle:       SimPos,
    /// The round's sampled trajectory — the ONE 3D unit direction the cone draw produced
    /// ([`ShotDir`]). A sim-space unit vector, never a pixel.
    pub trajectory:   ShotDir,
    /// The `(cell, *)` ground cell the round impacted — the tracer's terminus + the
    /// impact-flash position.
    pub impact_cell:  Cell,
    /// The storey [`Level`] of the impact (paired with [`impact_cell`](ShotFired::impact_cell)).
    pub impact_level: Level,
    /// What the round struck ([`ShotKind`]: ganger / cover / slab / ground / miss) — the
    /// presenter may branch the impact mark on it; a miss still draws muzzle + tracer.
    pub kind:         ShotKind,
}

impl ShotFired {
    /// Build a [`ShotFired`] for `shooter` from the round's already-computed
    /// [`ShotOutcome`](crate::resolve_coarse::ShotOutcome).
    ///
    /// Copies the geometry straight off the outcome — muzzle / trajectory / impact
    /// `(cell, level)` / kind — so the message EXPOSES the resolved trajectory without
    /// recomputing or changing any fire-result logic.
    #[must_use]
    pub const fn from_outcome(shooter: Entity, outcome: &ShotOutcome) -> Self {
        Self {
            shooter,
            muzzle: outcome.muzzle,
            trajectory: outcome.trajectory,
            impact_cell: outcome.cell,
            impact_level: outcome.level,
            kind: outcome.kind,
        }
    }
}
