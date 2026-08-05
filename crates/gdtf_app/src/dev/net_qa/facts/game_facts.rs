use super::{BattleActivity, StepperActivity};
use crate::dev::net_qa::wire::{AppPhaseNet, BattleScapePhaseNet};

crate::support_item! {
    /// What a game command's availability check may read about the host.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct GameFacts {
        phase:   AppPhaseNet,
        stepper: StepperActivity,
    }
}

impl GameFacts {
    pub(crate) const fn new(phase: AppPhaseNet, stepper: StepperActivity) -> Self {
        Self { phase, stepper }
    }

    crate::support_item! {
        /// Where the host is at every level of its state machine.
        #[must_use]
        const fn phase(self) -> AppPhaseNet {
            self.phase
        }
    }

    crate::support_item! {
        /// Whether the procgen stepper owns generation right now.
        #[must_use]
        const fn stepper(self) -> StepperActivity {
            self.stepper
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
}
