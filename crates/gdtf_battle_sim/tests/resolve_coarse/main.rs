//! Integration tests for the E2.9 coarse-pipeline entry point (GTW-172).
//!
//! These exercise the **public** `resolve_coarse` surface as a downstream user
//! would: build a battle (via `setup_battle` for the on-the-real-path geometry, or
//! hand-built occupancy / surface / cover for the precise miss / cover / ground
//! geometry), call `resolve_coarse`, and assert on the returned `ShotOutcome`.
//!
//! Every position the resolver touches is in **sim units** (cubic-voxel `SimPos` /
//! a unit-`Vec3` direction) — zero pixels. The cone width and concentration are
//! fed in already composed (the ticket's composed inputs); the geometry tests use
//! a **zero cone** so the sample is dead-center on the aim axis (deterministic) and
//! a HIGH occupant band so any round impacts, keeping the hand-computed geometry
//! robust against the tunable aim/muzzle fractions.

mod harness;
mod outcomes;
mod purity;
mod trajectories;
