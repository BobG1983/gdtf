//! Per-field ganger state as **separate ECS components** — the E1.2 decomposition.
//!
//! A ganger is not one monolithic struct. Each piece of its battle state is its
//! own Bevy [`Component`](bevy::prelude::Component) so a system can query **any
//! subset** without touching the others — change-detection, archetype filters, and
//! disjoint queries all work per field. The monolithic `GangerState` is
//! deliberately **not** modelled here (the GTW-6 architectural ruling); a system
//! that only cares about `Hp` queries `&Hp` alone, never a god-struct.
//!
//! Every value carries its meaning in its type (no-bare-types): a cubic-voxel
//! grid key is wrapped in [`Position`], a turn count in [`Tu`], and so on. The
//! newtypes use the E1.1 house style — a **private** inner field plus a derived
//! [`Deref`](bevy::prelude::Deref) (never a hand-written `impl Deref`) — and the
//! inner direction / stance / life kinds are **named domain enums**, not bare
//! primitives.
//!
//! [`Default`] gives each component its documented **structural** initial value
//! (a fresh, unhurt, standing ganger: [`LifeState::Alive`], [`StanceKind::Standing`],
//! aim off, zero counts). These are invariants of "a newly-spawned ganger", not
//! tunable balance magnitudes. See `docs/combat/combat.md`, `resolution.md`,
//! `wounds-and-roster.md`, and `stats.md`.
//!
//! GTW-201 code-health: this concern is a dir-module split by responsibility — the
//! grid position ([`position`]), the 8-way [`Direction`] compass + [`Facing`]
//! ([`direction`]), the posture / aim-mode / gang identity ([`stance`]), the
//! numeric pools & attribute stats ([`vitals`]), and the terminal life-state
//! machine ([`life`]). This `mod.rs` is wiring-only; every public path is preserved
//! via the re-exports below.

mod direction;
mod life;
mod position;
mod stance;
mod vitals;

#[cfg(test)]
mod test;

pub use direction::{Direction, Facing};
pub use life::{LifeState, Stabilized};
pub use position::Position;
pub use stance::{Aiming, Faction, Stance, StanceKind};
pub use vitals::{GangerName, Hp, Luck, Shooting, Toughness, Tu, TuMax, Wounds};
