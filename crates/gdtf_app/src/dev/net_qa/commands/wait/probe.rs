//! What each wait condition is read from, sampled once per frame.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::playback::PlaybackGate;
use gdtf_battle_sim::{act_log::ActLog, acts::movement::WalkInProgress, turn::TurnStarted};

use crate::{
    dev::net_qa::{
        facts::GameFactsParam,
        wire::{AppPhaseNet, AppPhaseTargetNet, WaitConditionNet},
    },
    states::running::game::battlescape::{
        battle_running::BattleRunningComplete, generation::GenerationComplete,
    },
};

/// Whether a wait's condition holds right now.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::dev::net_qa) struct ConditionMet(bool);

impl ConditionMet {
    /// Wrap whether the condition holds.
    pub(in crate::dev::net_qa) const fn new(met: bool) -> Self {
        Self(met)
    }
}

crate::support_item! {
    /// How many turn hand-offs this process has announced.
    #[derive(Resource, Deref, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    struct TurnChangeCount(u64);
}

impl TurnChangeCount {
    const fn count_one(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

crate::support_item! {
    /// Tally every `TurnStarted` so a wait can tell one that arrived after it was admitted.
    fn count_turn_changes(mut turns: MessageReader<TurnStarted>, mut seen: ResMut<TurnChangeCount>) {
        for _turn in turns.read() {
            seen.count_one();
        }
    }
}

/// Every source a wait condition resolves against.
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct WaitProbe<'w, 's> {
    facts:     GameFactsParam<'w>,
    gate:      PlaybackGate<'w>,
    log:       Option<Res<'w, ActLog>>,
    walking:   Query<'w, 's, (), With<WalkInProgress>>,
    turns:     Res<'w, TurnChangeCount>,
    decided:   Option<Res<'w, BattleRunningComplete>>,
    generated: Option<Res<'w, GenerationComplete>>,
}

impl WaitProbe<'_, '_> {
    /// Where the app is at every level of its state machine.
    pub(in crate::dev::net_qa) fn phase(&self) -> AppPhaseNet {
        self.facts.sample().phase()
    }

    /// The turn tally as it stands this frame.
    pub(in crate::dev::net_qa) fn turn_changes(&self) -> TurnChangeCount {
        *self.turns
    }

    /// Whether `condition` holds, measured against the tally the call parked with.
    pub(in crate::dev::net_qa) fn met(
        &self,
        condition: WaitConditionNet,
        parked_with: TurnChangeCount,
    ) -> ConditionMet {
        match condition {
            WaitConditionNet::CaughtUp => ConditionMet::new(self.gate.is_open()),
            WaitConditionNet::Phase(target) => phase_matches(target, self.phase()),
            WaitConditionNet::LogAtLeast(wanted) => {
                ConditionMet::new(self.log.as_deref().map_or(0, ActLog::len) >= *wanted)
            }
            WaitConditionNet::WalkComplete => ConditionMet::new(self.walking.is_empty()),
            WaitConditionNet::TurnChanged => ConditionMet::new(**self.turns > *parked_with),
            WaitConditionNet::BattleDecided => ConditionMet::new(self.decided.is_some()),
            WaitConditionNet::GenerationComplete => ConditionMet::new(self.generated.is_some()),
        }
    }
}

fn phase_matches(target: AppPhaseTargetNet, live: AppPhaseNet) -> ConditionMet {
    ConditionMet::new(
        target.app().is_none_or(|want| want == live.app())
            && target
                .running()
                .is_none_or(|want| live.running() == Some(want))
            && target.game().is_none_or(|want| live.game() == Some(want))
            && target
                .battlescape()
                .is_none_or(|want| live.battlescape() == Some(want))
            && target
                .aftermath()
                .is_none_or(|want| live.aftermath() == Some(want)),
    )
}
