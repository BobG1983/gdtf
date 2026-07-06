//! Tests of the flash tint + TTL (mirrors `flash.rs` / `readers.rs`).

use std::time::Duration;

use bevy::prelude::Alpha;
use gdtf_battle_sim::ganger::Wounds;

use super::super::{
    flash::{FLASH_SECONDS, FlashTtl},
    readers::bleed_tint,
};

/// `bleed_tint` is a strictly-DECREASING relation in remaining wounds: a ganger nearer
/// death (fewer wounds) bleeds a more opaque flash, never a pinned literal.
#[test]
fn bleed_tint_alpha_decreases_with_remaining_wounds() {
    let near_death = bleed_tint(Wounds::new(0)).alpha();
    let healthier = bleed_tint(Wounds::new(5)).alpha();
    assert!(
        near_death > healthier,
        "fewer remaining wounds must bleed a MORE opaque (higher alpha) flash: \
         {near_death} (0 wounds) must exceed {healthier} (5 wounds)",
    );
}

/// A fresh `FlashTtl` is not finished, and ticking it past `FLASH_SECONDS` finishes it —
/// the one-shot countdown `expire_flashes` keys its despawn on.
#[test]
fn flash_ttl_finishes_after_its_window() {
    let mut ttl = FlashTtl::new();
    // A zero tick does not finish a fresh one-shot timer.
    assert!(
        !ttl.tick(Duration::ZERO),
        "a fresh FlashTtl must not be finished before any time passes",
    );
    // Ticking past the full window finishes it.
    let past = Duration::from_secs_f32(FLASH_SECONDS + 0.1);
    assert!(
        ttl.tick(past),
        "ticking a FlashTtl past FLASH_SECONDS must finish it (the one-shot signal)",
    );
}
