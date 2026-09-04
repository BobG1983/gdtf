use bevy::prelude::*;
use gdtf_battle_sim::rng::BattleSeed;

use crate::states::RunningState;

crate::support_item! {
    /// Asks the app to leave the menu and start a battle.
    #[derive(Message, Debug, Clone)]
    struct StartBattleRequested {
        seed: Option<BattleSeed>,
    }
}

impl StartBattleRequested {
    crate::support_item! {
        /// Request a battle, optionally pinning the seed.
        #[must_use]
        const fn new(seed: Option<BattleSeed>) -> Self {
            Self { seed }
        }
    }

    pub(crate) const fn seed(&self) -> Option<BattleSeed> {
        self.seed
    }
}

pub(in crate::states::running::menu) fn apply_start_battle(
    mut requests: MessageReader<StartBattleRequested>,
    mut next: ResMut<NextState<RunningState>>,
    mut commands: Commands,
) {
    for request in requests.read() {
        if let Some(seed) = request.seed() {
            commands.insert_resource(seed);
        }
        next.set(RunningState::Game);
    }
}
