use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::{PendingImpact, PlaybackGate, Played, ShotProjectile};
use gdtf_battle_sim::shot_fired::ShotFired;

/// What the presenter is still showing of the shots the sim has resolved.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape::battle_running) struct ShotPresentation<'w, 's> {
    gate:        PlaybackGate<'w>,
    projectiles: Query<'w, 's, (), With<ShotProjectile>>,
    pending:     Query<'w, 's, (), With<PendingImpact>>,
    shots:       MessageReader<'w, 's, Played<ShotFired>>,
}

impl ShotPresentation<'_, '_> {
    /// Whether playback has caught up with the act log.
    pub(super) fn caught_up(&self) -> PlaybackCaughtUp {
        PlaybackCaughtUp::new(self.gate.is_open())
    }

    /// Whether a projectile or an unresolved impact is still on screen.
    pub(super) fn fx_busy(&self) -> FxBusy {
        FxBusy::new(!self.projectiles.is_empty() || !self.pending.is_empty())
    }

    /// Whether the presenter showed a shot since this was last read.
    pub(super) fn shot_shown(&mut self) -> ShotShown {
        ShotShown::new(self.shots.read().next().is_some())
    }
}

/// Whether playback has drained the act log the sim already resolved.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PlaybackCaughtUp(bool);

impl PlaybackCaughtUp {
    /// Wrap the gate answer.
    const fn new(caught_up: bool) -> Self {
        Self(caught_up)
    }
}

/// Whether shot FX are still on screen.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FxBusy(bool);

impl FxBusy {
    /// Wrap the busy answer.
    const fn new(busy: bool) -> Self {
        Self(busy)
    }
}

/// Whether a shot reached the screen since the last read.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ShotShown(bool);

impl ShotShown {
    /// Wrap the playback answer.
    const fn new(shown: bool) -> Self {
        Self(shown)
    }
}
