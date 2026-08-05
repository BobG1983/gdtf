use super::{BattleActivity, BattleModel, BattleScreen, PresenterReadiness};
use crate::dev::net_qa::wire::{AppPhaseNet, BattleScapePhaseNet, GamePhaseNet};

crate::support_item! {
    /// What a game command's availability check may read about the host.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct GameFacts {
        phase: AppPhaseNet,
        model: BattleModel,
    }
}

impl GameFacts {
    pub(crate) const fn new(phase: AppPhaseNet, model: BattleModel) -> Self {
        Self { phase, model }
    }

    crate::support_item! {
        /// Where the host is at every level of its state machine.
        #[must_use]
        const fn phase(self) -> AppPhaseNet {
            self.phase
        }
    }

    crate::support_item! {
        /// Whether a battle is running right now.
        #[must_use]
        const fn battle_activity(self) -> BattleActivity {
            match self.phase.battlescape() {
                Some(BattleScapePhaseNet::BattleRunning) => BattleActivity::Running,
                Some(_) | None => BattleActivity::NotRunning,
            }
        }
    }

    crate::support_item! {
        /// Whether the battle screen is up, at any battlescape phase.
        #[must_use]
        const fn battle_screen(self) -> BattleScreen {
            match self.phase.game() {
                Some(GamePhaseNet::BattleScape) => BattleScreen::Open,
                Some(_) | None => BattleScreen::Closed,
            }
        }
    }

    crate::support_item! {
        /// Whether a running battle also has its sim state loaded.
        #[must_use]
        const fn presenter_readiness(self) -> PresenterReadiness {
            match (self.battle_activity(), self.model) {
                (BattleActivity::Running, BattleModel::Present) => PresenterReadiness::Ready,
                (BattleActivity::Running, BattleModel::Absent)
                | (
                    BattleActivity::NotRunning,
                    BattleModel::Present | BattleModel::Absent,
                ) => PresenterReadiness::NotReady,
            }
        }
    }
}
