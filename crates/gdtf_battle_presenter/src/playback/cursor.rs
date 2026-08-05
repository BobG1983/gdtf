//! Act-log playback cursor and hold phases.

use std::time::Duration;

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::act_log::{ActLog, ActSeq};

/// Whether the FX pipeline has been observed busy during an impact wait.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FxSeenBusy(bool);

impl FxSeenBusy {
    /// Build from a seen-busy flag.
    #[must_use]
    pub const fn new(seen: bool) -> Self {
        Self(seen)
    }
}

/// Count of act-log entries skipped by a catch-up jump.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkippedActs(u32);

impl SkippedActs {
    /// Build from a skip count.
    #[must_use]
    pub const fn new(skipped: u32) -> Self {
        Self(skipped)
    }

    /// Add to the skip count, saturating at `u32::MAX`.
    pub const fn add(&mut self, count: u32) {
        self.0 = self.0.saturating_add(count);
    }
}

/// How the cursor is currently holding before the next act.
#[derive(Debug, Clone)]
pub enum ActHoldPhase {
    /// Fixed-duration dwell timer.
    Timed {
        /// Time remaining on the dwell.
        remaining: Timer,
    },
    /// Wait for projectile/impact FX, then a short beat.
    AwaitingImpact {
        /// Whether the FX pipeline was seen busy at least once.
        seen_busy: FxSeenBusy,
        /// Hard cap on how long to wait for impact.
        cap:       Timer,
        /// Beat after impact (or cap) before releasing.
        beat:      Duration,
    },
}

/// Active hold attached to the playback cursor.
#[derive(Debug, Clone)]
pub struct ActHold {
    phase: ActHoldPhase,
}

impl ActHold {
    /// Hold for a fixed number of seconds.
    #[must_use]
    pub fn timed(seconds: f32) -> Self {
        Self {
            phase: ActHoldPhase::Timed {
                remaining: Timer::from_seconds(seconds.max(0.0), TimerMode::Once),
            },
        }
    }

    /// Hold until impact FX clears or `cap_seconds`, then beat for `beat_seconds`.
    #[must_use]
    pub fn awaiting_impact(cap_seconds: f32, beat_seconds: f32) -> Self {
        Self {
            phase: ActHoldPhase::AwaitingImpact {
                seen_busy: FxSeenBusy::new(false),
                cap:       Timer::from_seconds(cap_seconds.max(0.0), TimerMode::Once),
                beat:      Duration::from_secs_f32(beat_seconds.max(0.0)),
            },
        }
    }

    /// Current hold phase.
    #[must_use]
    pub const fn phase(&self) -> &ActHoldPhase {
        &self.phase
    }

    /// Advance the hold by `delta`. Returns `None` when the hold is finished.
    #[must_use]
    pub fn advanced(mut self, delta: Duration, pipeline_busy: bool) -> Option<Self> {
        match &mut self.phase {
            ActHoldPhase::Timed { remaining } => {
                remaining.tick(delta);
                if remaining.is_finished() {
                    return None;
                }
            }
            ActHoldPhase::AwaitingImpact {
                seen_busy,
                cap,
                beat,
            } => {
                cap.tick(delta);
                if pipeline_busy {
                    *seen_busy = FxSeenBusy::new(true);
                } else if **seen_busy || cap.is_finished() {
                    let beat = *beat;
                    self.phase = ActHoldPhase::Timed {
                        remaining: Timer::new(beat, TimerMode::Once),
                    };
                    return Some(self);
                }
                if cap.is_finished() {
                    let beat = *beat;
                    self.phase = ActHoldPhase::Timed {
                        remaining: Timer::new(beat, TimerMode::Once),
                    };
                }
            }
        }
        Some(self)
    }
}

/// Where the view has read in the act log, and any active hold.
#[derive(Resource, Debug, Default)]
pub struct PlaybackCursor {
    shown:   ActSeq,
    holding: Option<ActHold>,
    skipped: SkippedActs,
}

impl PlaybackCursor {
    /// Sequence of the next act the view will show.
    #[must_use]
    pub const fn shown(&self) -> ActSeq {
        self.shown
    }

    /// Whether a hold is currently active.
    #[must_use]
    pub const fn is_holding(&self) -> bool {
        self.holding.is_some()
    }

    /// Active hold, if any.
    #[must_use]
    pub const fn hold(&self) -> Option<&ActHold> {
        self.holding.as_ref()
    }

    /// Acts skipped by catch-up jumps.
    #[must_use]
    pub const fn skipped(&self) -> SkippedActs {
        self.skipped
    }

    /// Reset to the start of the log with no hold.
    pub fn reset(&mut self) {
        self.shown = ActSeq::START;
        self.holding = None;
        self.skipped = SkippedActs::default();
    }

    /// Begin holding with the given phase.
    pub const fn hold_for(&mut self, hold: ActHold) {
        self.holding = Some(hold);
    }

    /// Tick the active hold. Returns `true` when the cursor is free to advance.
    pub fn tick_hold(&mut self, delta: Duration, pipeline_busy: bool) -> bool {
        match self.holding.take() {
            None => true,
            Some(hold) => {
                self.holding = hold.advanced(delta, pipeline_busy);
                self.holding.is_none()
            }
        }
    }

    /// Move past the act just shown.
    pub const fn advance_past_shown(&mut self) {
        self.shown = self.shown.next();
    }

    /// Jump the cursor to `oldest`, counting skipped acts.
    pub fn jump_to(&mut self, oldest: ActSeq) {
        self.skipped
            .add(u32::try_from(oldest.distance_from(self.shown)).unwrap_or(u32::MAX));
        self.shown = oldest;
    }
}

/// The act log being played and where the view has read to in it.
#[derive(SystemParam)]
pub struct LogPlayhead<'w> {
    pub(super) log:    Option<Res<'w, ActLog>>,
    pub(super) cursor: ResMut<'w, PlaybackCursor>,
}

impl LogPlayhead<'_> {
    /// Oldest retained and newest sequence, when a log is loaded.
    #[must_use]
    pub fn span(&self) -> Option<(ActSeq, ActSeq)> {
        self.log.as_ref().map(|log| (log.oldest_seq(), log.head()))
    }
}
