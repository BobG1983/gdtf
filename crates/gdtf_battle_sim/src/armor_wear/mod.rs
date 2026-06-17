//! Armor wear persistence + the armor-broken signal — the E3.5 slice (GTW-187).
//!
//! This is the **wear-side primitive** of per-hit application (`docs/combat/
//! weapons-and-armor.md` §"Per-hit resolution" step 3): a hit's computed
//! [`IntegrityWear`](crate::resolve_hit::IntegrityWear) (the E3.3
//! [`crate::resolve_hit::HitResult::wear`]) is subtracted from the struck location's
//! [`crate::armor::ArmorIntegrity`] on the **battle-local**
//! [`WornArmor`](crate::armor::WornArmor) copy, so a piece worn to `integrity ≤ 0` stops
//! protecting for the rest of the battle (later hits on that location resolve as
//! bare flesh — the [`WornArmor::protects`](crate::armor::WornArmor::protects) gate). The
//! worn copy is battle-local (E1.3): wearing it **never** touches the roster
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
//! deterministic and unit-testable with no Bevy app; the caller (E3.6's
//! `apply_hit`, and the headless test here) writes the returned `Some` to a
//! [`bevy::prelude::MessageWriter<ArmorBroken>`] at the system boundary. Pure
//! model logic — no renderer, no pixel.

#[cfg(test)]
mod test;
mod wear;

pub use wear::{ArmorBroken, wear_armor};
