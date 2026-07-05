//! [`setup_battle`] — pour a [`Situation`](crate::situation::Situation) into the
//! battle ECS — plus its [`BattleSetup`] result and the
//! [`has_stacked_gangers`] sanity helper.

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
pub use registries::{BattleRegistries, BattleSetup};
pub use resolve::has_stacked_gangers;
