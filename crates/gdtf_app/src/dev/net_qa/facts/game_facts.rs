use crate::dev::net_qa::wire::AppPhaseNet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GameFacts {
        phase: AppPhaseNet,
}

impl GameFacts {
        pub(crate) const fn new(phase: AppPhaseNet) -> Self {
        Self { phase }
    }

        pub(crate) const fn phase(self) -> AppPhaseNet {
        self.phase
    }
}
