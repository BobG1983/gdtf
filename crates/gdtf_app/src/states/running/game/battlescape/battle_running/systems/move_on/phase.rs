use bevy::prelude::*;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::states::running::game::battlescape::battle_running) struct SeenBusy(bool);

impl SeenBusy {
    pub(in crate::states::running::game::battlescape::battle_running) const fn new(
        seen: bool,
    ) -> Self {
        Self(seen)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(in crate::states::running::game::battlescape::battle_running) struct EndBackstopSeconds(f32);

impl EndBackstopSeconds {
    pub(in crate::states::running::game::battlescape::battle_running) const DEFAULT: f32 = 8.0;

    pub(in crate::states::running::game::battlescape::battle_running) const fn new(
        seconds: f32,
    ) -> Self {
        Self(seconds)
    }
}

impl Default for EndBackstopSeconds {
    fn default() -> Self {
        Self::new(Self::DEFAULT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::states::running::game::battlescape::battle_running) enum EndPhase {
    AwaitingDecidingShot { seen_busy: SeenBusy },
    NoDecidingShot,
}

#[derive(Resource, Debug)]
pub(in crate::states::running::game::battlescape::battle_running) struct EndTransition {
    phase:    EndPhase,
    backstop: Timer,
}

impl EndTransition {
    pub(in crate::states::running::game::battlescape::battle_running) fn new(
        phase: EndPhase,
    ) -> Self {
        Self::with_backstop(phase, EndBackstopSeconds::default())
    }

    pub(in crate::states::running::game::battlescape::battle_running) fn with_backstop(
        phase: EndPhase,
        backstop: EndBackstopSeconds,
    ) -> Self {
        Self {
            phase,
            backstop: Timer::from_seconds(*backstop, TimerMode::Once),
        }
    }

    pub(in crate::states::running::game::battlescape::battle_running) const fn phase(
        &self,
    ) -> EndPhase {
        self.phase
    }

    pub(in crate::states::running::game::battlescape::battle_running) fn tick(
        &mut self,
        delta: std::time::Duration,
    ) {
        self.backstop.tick(delta);
    }

    pub(in crate::states::running::game::battlescape::battle_running) fn backstop_elapsed(
        &self,
    ) -> bool {
        self.backstop.is_finished()
    }

    pub(in crate::states::running::game::battlescape::battle_running) const fn mark_seen_busy(
        &mut self,
    ) {
        if let EndPhase::AwaitingDecidingShot { seen_busy } = &mut self.phase {
            *seen_busy = SeenBusy::new(true);
        }
    }
}
