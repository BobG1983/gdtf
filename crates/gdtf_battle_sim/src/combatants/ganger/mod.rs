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
//! numeric pools & computed combat stats ([`vitals`]), the eight raw direct
//! [`attributes`] (GTW-384, the slowly-changing potential the computed stats derive
//! from), the [`derive_stats`] pure derivation (GTW-384), and the terminal life-state
//! machine ([`life`]). This `mod.rs` is wiring-only; every public path is preserved
//! via the re-exports below.

mod attributes;
mod derive_stats;
mod direction;
mod gang;
pub(crate) mod injury_projection;
mod life;
mod position;
pub(crate) mod rederive;
mod stance;
mod suppression;
mod vitals;

#[cfg(test)]
mod test;

pub use attributes::{Aim, Cool, GangerAttributes, Grit, Reflexes, Speed, Strength};
pub use derive_stats::{DerivedStats, derive_stats};
pub use direction::{Direction, Facing};
pub use gang::{GangMember, GangName, GangRegistry, GangRoster};
pub use injury_projection::{derive_stats_with_injuries, effective_luck, effective_toughness};
pub use life::{LifeState, Stabilized};
pub use position::Position;
pub use rederive::{rederive_stats_on_injury_change, rederive_stats_on_tuning_change};
pub use stance::{Aiming, Faction, Stance, StanceKind};
pub use suppression::{Suppressed, SuppressorCell};
pub use vitals::{
    Bottle, Fight, GangerName, Hp, HpMax, Luck, Morale, Reactions, Shooting, Toughness, Tu, TuMax,
    Wounds, WoundsMax,
};
