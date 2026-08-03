//! AoE / hit-type integration tests.
//!
//! `AoE` damages every occupant in radius: a weapon authored with `HitType::Blast`.
//!
//! Harness note (the `melee_act` / `melee_cover_smash` idiom): the sim crate is the low-level
//! act layer under test.

mod harness;
mod hit_types;
