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
//! - [`SimAppBuilder`] / [`insert_sim_resources`] / [`full_vision`] — the knobbed
//!   headless sim-app builder and the ONE canonical sim-resource seeding litany
//!   (GTW-576), plus the value-level RNG/ledger knobs ([`shot_rng`] /
//!   [`severity_rng`] / [`injury_rng`] / [`fight_rng`] / [`empty_slab_ledger`]).
//! - [`GangerEntityBuilder`] / [`target_bundle`] / [`single_mode`] / [`wield`] —
//!   the opt-in ganger-entity spawner + the canonical target bundle, fire-mode
//!   fixture, and wielded-weapon relater.
//!
//! # The "new test" recipe (GTW-576, P11)
//!
//! Adding a test for a new act / consequence family should never re-spell the
//! seeding litany or a fixture struct:
//!
//! 1. **App**: `SimAppBuilder::new().with_acts().build()` for a dispatch-band
//!    harness (add `.with_full_vision()` if the test routes moves;
//!    `.with_seed(..)` to pin a stream; `.with_battle().with_registries()` for a
//!    `setup_battle` lifecycle harness). Extra plugins / populated grids ride
//!    `app.add_plugins(..)` / `app.insert_resource(..)` on the built app.
//! 2. **Actors**: `GangerEntityBuilder::new().at(..).faction(..).spawn(world)`
//!    with only the knobs the system under test reads; arm it via
//!    [`wield`]`(world, ganger, bundle)`. Targets: `.combat_vitals(hp, wounds)`
//!    (or spawn [`target_bundle`] directly).
//! 3. **Fixtures**: never write an exhaustive `WeaponSpec` / `MeleeWeaponSpec`
//!    literal — use struct-update over [`test_weapon_spec`] /
//!    [`test_melee_weapon_spec`], overriding ONLY the load-bearing fields
//!    (`WeaponSpec { shove: Shove::new(true), ..test_weapon_spec() }`), so new
//!    spec fields are absorbed here, not at every call site.
//! 4. **Seeds**: a `World`/`SystemState` test draws through [`shot_rng`] /
//!    [`severity_rng`] / [`injury_rng`] — never `ShotRng::from_root(..)` inline.

mod actor;
mod ganger;
mod harness;
mod registries;
mod seeds;
mod situation;

pub use actor::{GangerEntityBuilder, single_mode, target_bundle, wield};
pub use ganger::{GangerSpawnBuilder, ganger_at};
pub use harness::{SimAppBuilder, TEST_PLAYER_GANG, TEST_SEED, full_vision, insert_sim_resources};
pub use registries::{
    TEST_ARMOR_KEY, TEST_MELEE_WEAPON_KEY, TEST_WEAPON_KEY, arbitrary_armor, key,
    test_armor_registry, test_armor_spec, test_melee_weapon_registry, test_melee_weapon_spec,
    test_terrain_registry, test_weapon_registry, test_weapon_spec,
};
pub use seeds::{empty_slab_ledger, fight_rng, injury_rng, reaction_rng, severity_rng, shot_rng};
pub use situation::{SituationBuilder, fixtures, test_gang_registry, test_pieces, wall_at};
