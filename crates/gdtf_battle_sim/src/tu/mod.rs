//! The **TU-economy primitives** the rest of E4 spends through — model-authoritative
//! Time-Unit bookkeeping.
//!
//! Three pure verbs over a ganger's [`Tu`](crate::ganger::Tu) (the current pool) and
//! [`TuMax`](crate::ganger::TuMax) (the round-start maximum): [`can_spend_tu`] (does the
//! pool afford a cost), [`spend_tu`] (saturating-subtract a cost from the pool — never
//! underflows), and [`reset_tu`] (restore the pool to its maximum at round start). The
//! cost is itself a [`Tu`](crate::ganger::Tu) (the named TU amount — no bare `u8` crosses
//! the boundary).
//!
//! These are **pure math, no [`World`](bevy::ecs::world::World) access** — they take and
//! return the landed ganger newtypes, so they unit-test against bare values with no ECS
//! plumbing. resolution.md §"What's pure math vs sim": *"TU bookkeeping is
//! model-authoritative too (`spend_tu` / `reset_tu` / `can_spend_tu`)."* All arithmetic is
//! **saturating** on the unsigned `u8` pool — spending more than is left floors at `0`,
//! never wraps to `~255`.
//!
//! The COST **magnitudes** are tuning, sourced by the later E4 slices (E4.1 stance/turn
//! costs, E4.4 fire costs); this slice ships the **mechanism only** — the three verbs plus
//! [`TuMax`](crate::ganger::TuMax).

mod economy;
#[cfg(test)]
mod test;

pub use economy::{can_spend_tu, reset_tu, spend_tu};
