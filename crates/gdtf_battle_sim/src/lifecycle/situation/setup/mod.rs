mod armor_scenes;
mod ganger_scene;
mod orchestrate;
mod registries;
mod resolve;
mod seed_cover;
mod seed_slabs;
mod spawn_gangers;
mod weapon_scenes;

pub use orchestrate::setup_battle;
pub use registries::{BattleRegistries, BattleSetup, GangerCount};
pub use resolve::{StackedGangers, has_stacked_gangers};
