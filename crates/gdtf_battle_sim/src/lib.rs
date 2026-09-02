//! Authoritative, render-free combat simulation for GDTF turn-based battles.

pub mod act_log;
pub mod acts;
pub mod ai;
pub mod combatants;
pub mod damage_resolution;
pub mod effects;
pub mod equipment;
pub mod falls;
pub mod foundation;
pub mod level;
pub mod lifecycle;
pub mod melee;
pub mod perception;
pub mod reaction;
pub mod shot_pipeline;
pub mod suppression;
pub mod terrain;
pub mod turn;

pub mod prelude;
pub mod test_support;
pub mod tuning;

pub use combatants::{faced_cell, ganger, posture, tu};
pub use damage_resolution::{
    apply_hit, hit_location, inflicted_wound, injuries, matchup, resolve_and_apply, resolve_hit,
    severity,
};
pub use equipment::{armor, armor_wear, magazine, weapon};
pub use foundation::{metric, registry, rng};
pub use lifecycle::{battle, procgen, situation};
pub use perception::{los, pathfinder, peek_sync, visibility};
pub use shot_pipeline::{
    aim, aoe, central_axis, clearance, cone, fire, march, resolve_coarse, sample_cone, shot_fired,
    stability,
};
pub use terrain::{
    cover, def, emplacement, entity, floor, occupancy, occupancy_sync, openable, piece, slab,
    successor, surface, vertical,
};
