//! Transient FX flashes (GTW-48 S6 / GTW-220): the presenter's one-shot FX layer.
//!
//! This module turns the three already-landed sim FX MESSAGES into short-lived 16x16
//! FX sprites drawn from the effects sheet ([`SheetRole::Effects`](crate::SheetRole),
//! `assets/tiles/alt_tileset_effects.png`):
//!
//! - [`Bleeding`](gdtf_battle_sim::Bleeding) `{ ganger }` -> a blood/hit FLASH sprite at
//!   the ganger's cell ([`read_bleeding`]); the flash's intensity is a RELATION to the
//!   ganger's [`Wounds`](gdtf_battle_sim::Wounds) (the message carries NO amount — verified
//!   `bleed.rs:62-65`), read from a `Query<&Wounds>`, never a pinned literal.
//! - [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) `{ ganger, part }` -> a spark/break flash
//!   at the ganger's cell ([`read_armor_broken`]).
//! - [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) `{ at }` -> a debris/rubble burst at
//!   `cell_to_world(at)` ([`read_cover_destroyed`]); ADDITIVE to the S4 rubble swap.
//!
//! Every spawned flash carries a [`FlashTtl`] lifetime + an [`FxFlash`] marker; the
//! [`expire_flashes`] system ticks each [`FlashTtl`] with [`Res<Time>`] and despawns the
//! flash on expiry — THAT expiry is what makes the flashes one-shot / transient. Multiple
//! messages for the same ganger/cell in one frame each spawn an INDEPENDENT short-lived
//! sprite (NO coalescing this slice).
//!
//! WHICH effect tile each FX draws is DATA-DRIVEN: a per-line-commented
//! `assets/tiles/effect_roles.ron`, loaded through the SAME generic
//! [`RonAsset<T>`](gdtf_assets::RonAsset) loader S4's `tile_roles.ron` uses, mapping each FX
//! to a [`TileIndex`](crate::TileIndex). Nothing about the index choices is hardcoded in
//! Rust.
//!
//! It only READS the three sim messages (+ a `Query<&Position>` / `Query<&Wounds>`) and adds
//! ZERO sim setup/teardown. It mirrors, never owns, combat truth — the one-way
//! `input -> presenter -> sim` edge (ADR-0001); the sim never reads the presenter.

mod flash;
mod readers;
mod roles;

#[cfg(test)]
mod test;

pub use flash::{FlashTtl, FxFlash, expire_flashes};
pub use readers::{read_armor_broken, read_bleeding, read_cover_destroyed};
pub use roles::{EffectRoles, EffectRolesHandle, load_effect_roles, resolve_effect_roles};
