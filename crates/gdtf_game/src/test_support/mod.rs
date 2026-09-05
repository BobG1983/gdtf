//! Test helpers and re-exports of scene markers under `test-support`.

mod battle_app;
mod markers;
mod register;

pub use battle_app::BattleAppBuilder;
pub use markers::*;
pub use register::{
    app_state, load_released, register_headless, register_scenes_with_default_plugins,
    seed_load_gate,
};
