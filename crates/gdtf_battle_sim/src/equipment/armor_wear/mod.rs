//! Armor wear persistence + the armor-broken signal — the E3.5 slice (GTW-187).
//!
//! This is the **wear-side primitive** of per-hit application (`docs/combat/
//! weapons-and-armor.md` §"Per-hit resolution" step 3): a hit's computed
//! [`IntegrityWear`](crate::resolve_hit::IntegrityWear) (the E3.3
//! [`crate::resolve_hit::HitResult::wear`]) is subtracted from the struck location's
//! **battle-local armor-piece ENTITY** [`crate::armor::ArmorIntegrity`] component
//! (resolved via [`Wears`](crate::armor::Wears), ADR-0004), so a piece worn to
//! `integrity ≤ 0` stops protecting for the rest of the battle (later hits on that
//! location resolve as bare flesh — the `integrity > 0` gate). The piece entity is
//! battle-local (`linked_spawn`, E1.3): wearing it **never** touches the roster
//! [`crate::armor::SourceArmor`] (the model/view separation; ADR-0001,
//! `docs/decisions/0001-rust-bevy-rewrite.md`).
//!
//! When a piece crosses from protecting (`integrity > 0`) to broken
//! (`integrity ≤ 0`) — **exactly once**, on that single crossing — an
//! [`ArmorBroken`] signal is emitted carrying the struck
//! [`BodyPart`](crate::armor::BodyPart) and the owning ganger
//! [`Entity`](bevy::prelude::Entity). A piece already at `≤ 0` that re-wears does NOT
//! re-emit (the depletion fired once already). This mirrors the
//! [`crate::occupancy_sync::CoverDestroyed`] precedent
//! (`docs/combat/resolution.md` §3's cover-destroyed signal): a buffered Bevy
//! **message** (`#[derive(Message)]` — Bevy 0.18 renamed buffered
//! `Event`/`EventReader` to `Message`/`MessageReader`, `bevy-traps.md` #4), NOT
//! the targeted/observer `Event` API.
//!
//! The crossing-detection lives in the **pure** [`wear_armor`] helper so it is
//! deterministic and unit-testable with no Bevy app; it returns an
//! [`ArmorWearOutcome`] ([`Unaffected`](ArmorWearOutcome::Unaffected) /
//! [`Damaged`](ArmorWearOutcome::Damaged) carrying the per-hit integrity delta /
//! [`Broke`](ArmorWearOutcome::Broke)), which the caller (E3.6's `apply_hit`)
//! carries unchanged as the hit report's `wear` outcome (GTW-573) — surfaced to the
//! presenter on [`ShotFired`](crate::shot_fired::ShotFired)`.report` (GTW-313). Pure model
//! logic — no renderer, no pixel.

#[cfg(test)]
mod test;
mod wear;

pub use wear::{ArmorBroken, ArmorDamaged, ArmorWearOutcome, wear_armor};
