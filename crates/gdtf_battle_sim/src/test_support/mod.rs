//! The CANONICAL shared test builders for the combat sim — the ONE source of
//! truth for test gangers, situations, and the weapon/armor specs + registries
//! they resolve against.
//!
//! This module is compiled under `#[cfg(any(test, feature = "test-support"))]`
//! (see [`crate`]'s `lib.rs`): the sim's OWN `#[cfg(test)]` unit tests see it for
//! free, and a downstream crate (`gdtf_app` / `gdtf_battle_input` /
//! `gdtf_battle_presenter` / `gdtf_test_utils`) reaches it by depending on this
//! crate with the `test-support` feature on. Because the sim is the LOW-LEVEL crate
//! (its only deps are `bevy` / `rand` / `serde`), the canonical builders live HERE —
//! downstream crates share ONE builder without the sim depending "up". The binary
//! build (`cargo dbuild`) enables NEITHER `test` nor `test-support`, so this module
//! is cfg'd OUT of the binary entirely (no production code references it).
//!
//! The surface, all re-exported from this `mod`:
//!
//! - [`GangerSpawnBuilder`] / [`ganger_at`] — the fluent ganger builder with sane
//!   test defaults + per-field overrides, and the thin `(at, faction)` free fn.
//! - [`SituationBuilder`] + [`fixtures`] — the fluent battlefield builder + the
//!   named multi-ganger fixtures ([`two_ganger`](fixtures::two_ganger),
//!   [`one_player_two_enemies`](fixtures::one_player_two_enemies),
//!   [`player_only`](fixtures::player_only),
//!   [`minimal_with_cells`](fixtures::minimal_with_cells)).
//! - [`test_weapon_registry`] / [`test_armor_registry`] / [`test_weapon_spec`] /
//!   [`test_armor_spec`] / [`arbitrary_armor`] / [`key`] / [`TEST_WEAPON_KEY`] /
//!   [`TEST_ARMOR_KEY`] — the shared specs + registries + the `(cell, level)` key
//!   helper consolidated out of the former per-module test-support copies.

mod ganger;
mod registries;
mod situation;

pub use ganger::{GangerSpawnBuilder, ganger_at};
pub use registries::{
    TEST_ARMOR_KEY, TEST_WEAPON_KEY, arbitrary_armor, key, test_armor_registry, test_armor_spec,
    test_terrain_registry, test_weapon_registry, test_weapon_spec,
};
pub use situation::{SituationBuilder, fixtures, test_gang_registry, test_pieces, wall_at};
