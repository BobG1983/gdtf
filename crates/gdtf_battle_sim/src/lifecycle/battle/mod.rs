//! Battle lifecycle: setup, teardown, outcome checks, and in-progress state.

mod messages;
mod outcome;
mod plugin;
mod resources;
mod runtime_seed;
mod setup;
mod teardown;

#[cfg(test)]
mod test;

pub use messages::{
    BattleLost, BattleReady, BattleWon, SetupBattleRequested, TeardownBattleRequested,
};
pub use outcome::check_outcome;
pub use plugin::BattleSimPlugin;
pub use resources::{BattleInProgress, BattleRoster, PlayerFaction};
pub use setup::setup_battle_on_request;
pub use teardown::teardown_battle_on_request;
