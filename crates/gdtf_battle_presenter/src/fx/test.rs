//! Unit tests for the transient FX-flash layer.

use std::time::Duration;

use bevy::prelude::Alpha;
use gdtf_battle_sim::Wounds;

use super::{
    flash::{FLASH_SECONDS, FlashTtl},
    readers::bleed_tint,
    roles::EffectRoles,
};

/// The shipped `effect_roles.ron` parses into `EffectRoles` and exposes every FX role —
/// a `ron::de` round-trip of the SHIPPED bytes.
///
/// It asserts the file PARSES and HAS all three FX roles (a missing field is a deserialize
/// error); it does NOT pin a tunable index magnitude (those are data the engineer eyeballs
/// and may adjust). A light distinctness guard catches an all-collapsed authoring slip.
#[test]
fn shipped_effect_roles_ron_parses_with_all_roles() {
    const SHIPPED: &str = include_str!("../../../../assets/tiles/effect_roles.ron");
    let parsed: Result<EffectRoles, _> = ron::de::from_str(SHIPPED);
    assert!(
        parsed.is_ok(),
        "shipped effect_roles.ron must parse into EffectRoles, got: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(roles) = parsed else {
        return;
    };
    // The three FX roles must not all collapse onto one index (an authoring slip) — a
    // structural guard, not a magnitude pin.
    let all_same = roles.bleed == roles.armor_break && roles.armor_break == roles.cover_destroyed;
    assert!(
        !all_same,
        "the three FX roles must not all share one index (authoring slip)",
    );
}

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
