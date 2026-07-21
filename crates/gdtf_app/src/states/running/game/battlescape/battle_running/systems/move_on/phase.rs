//! [`EndTransition`] — the per-run latch the deferred end-of-battle gate carries from the
//! outcome-decided frame to the moment it is safe to leave (GTW-334, extended GTW-727
//! C40).

use bevy::prelude::*;

/// Whether the FX pipeline has been observed BUSY since the end latched.
///
/// A named flag (`no-bare-types.md`), never a bare `bool`: a projectile is spawned through
/// a DEFERRED scene spawn, so it is not queryable on the frame its message was drained. An
/// idle read taken in that gap is the PRE-SPAWN gap, not a drained pipeline — so idle only
/// counts as drained once busy has been seen first.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::states::running::game::battlescape::battle_running) struct SeenBusy(bool);

impl SeenBusy {
    /// Build the guard from whether the pipeline has been seen busy yet.
    pub(in crate::states::running::game::battlescape::battle_running) const fn new(
        seen: bool,
    ) -> Self {
        Self(seen)
    }
}

/// How long the end transition will wait, at most, before leaving anyway (GTW-727 C40).
///
/// A named newtype over the timeout in seconds.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(in crate::states::running::game::battlescape::battle_running) struct EndBackstopSeconds(f32);

impl EndBackstopSeconds {
    /// The shipped backstop — `8` seconds.
    ///
    /// Generous enough that it never truncates a real ending: the longest legitimate wait
    /// is the playback cursor draining whatever the killing exchange left in the act log,
    /// plus the deciding bolt's flight. Short enough that a wait which CANNOT complete ends
    /// the battle instead of stranding the player, which matters because the end transition
    /// runs with input gated shut and there is no in-battle quit key — a phase that can
    /// hold forever is a hang with no way out but killing the process.
    pub(in crate::states::running::game::battlescape::battle_running) const DEFAULT: f32 = 8.0;

    /// Build a backstop timeout from a duration in seconds.
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

/// What the end transition is waiting for.
///
/// A typed phase (the no-bare-types discipline): a bare "should wait" flag could not tell
/// the flee / non-shot end (leave at once) from an in-flight deciding shot (wait for the
/// tracer) apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::states::running::game::battlescape::battle_running) enum EndPhase {
    /// A deciding shot WAS in flight when the outcome was decided: the FX pipeline is
    /// expected to go busy (spawn the bolt) then drain (land the impact).
    AwaitingDecidingShot {
        /// Whether the pipeline has been observed busy since the latch.
        seen_busy: SeenBusy,
    },
    /// NO deciding shot was in flight (a flee, a bleed-out census, any non-projectile end):
    /// there is no tracer to wait for.
    NoDecidingShot,
}

/// The per-run end-transition latch: what the gate is waiting for, and how much longer it
/// is willing to wait at all.
///
/// A state-scoped [`Resource`] (`bevy-traps.md` #1): inserted by
/// [`move_on`](super::move_on) the first frame the run-complete marker appears, and removed
/// `OnExit(BattleRunning)` alongside that marker.
#[derive(Resource, Debug)]
pub(in crate::states::running::game::battlescape::battle_running) struct EndTransition {
    /// What the transition is waiting for.
    phase:    EndPhase,
    /// The hard upper bound on waiting (GTW-727 C40).
    backstop: Timer,
}

impl EndTransition {
    /// Build a latch in `phase`, with the default backstop.
    pub(in crate::states::running::game::battlescape::battle_running) fn new(
        phase: EndPhase,
    ) -> Self {
        Self::with_backstop(phase, EndBackstopSeconds::default())
    }

    /// Build a latch in `phase` with an explicit backstop (the value tests inject).
    pub(in crate::states::running::game::battlescape::battle_running) fn with_backstop(
        phase: EndPhase,
        backstop: EndBackstopSeconds,
    ) -> Self {
        Self {
            phase,
            backstop: Timer::from_seconds(*backstop, TimerMode::Once),
        }
    }

    /// What the transition is waiting for.
    pub(in crate::states::running::game::battlescape::battle_running) const fn phase(
        &self,
    ) -> EndPhase {
        self.phase
    }

    /// Advance the backstop by `delta`.
    pub(in crate::states::running::game::battlescape::battle_running) fn tick(
        &mut self,
        delta: std::time::Duration,
    ) {
        self.backstop.tick(delta);
    }

    /// Whether the backstop has elapsed — the transition must leave now, whatever it was
    /// waiting for.
    pub(in crate::states::running::game::battlescape::battle_running) fn backstop_elapsed(
        &self,
    ) -> bool {
        self.backstop.is_finished()
    }

    /// Record that the FX pipeline has been observed busy.
    pub(in crate::states::running::game::battlescape::battle_running) const fn mark_seen_busy(
        &mut self,
    ) {
        if let EndPhase::AwaitingDecidingShot { seen_busy } = &mut self.phase {
            *seen_busy = SeenBusy::new(true);
        }
    }
}
