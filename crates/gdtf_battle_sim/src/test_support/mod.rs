//! Shared builders, harnesses, and registries for unit and integration tests.

mod actor;
mod ganger;
mod harness;
mod literals;
mod registries;
mod seeds;
mod situation;
mod terrain;

pub use actor::{GangerEntityBuilder, single_mode, target_bundle, wield};
pub use ganger::{GangerSpawnBuilder, ganger_at};
pub use harness::{
    SimAppBuilder, SimAppPart, TEST_PLAYER_GANG, TEST_SEED, full_vision, insert_sim_resources,
};
pub use literals::{dot_turns, field_turns};
pub use registries::{
    TEST_ARMOR_KEY, TEST_MELEE_WEAPON_KEY, TEST_MOUNTED_WEAPON_KEY, TEST_WEAPON_KEY,
    arbitrary_armor, key, test_armor_registry, test_armor_spec, test_melee_weapon_registry,
    test_melee_weapon_spec, test_weapon_registry, test_weapon_spec,
};
pub use seeds::{empty_slab_ledger, fight_rng, injury_rng, reaction_rng, severity_rng, shot_rng};
pub use situation::{
    SituationBuilder, emplacement_at, fixtures, setup_request, test_gang_registry, test_pieces,
    wall_at,
};
pub use terrain::{UNRESOLVABLE_SPRITE, test_terrain_registry};
