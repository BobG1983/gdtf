use std::time::Duration;

use bevy::prelude::*;
use gdtf_battle_sim::act_log::ActSeq;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FxSeenBusy(bool);

impl FxSeenBusy {
        #[must_use]
    pub const fn new(seen: bool) -> Self {
        Self(seen)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkippedActs(u32);

impl SkippedActs {
        #[must_use]
    pub const fn new(skipped: u32) -> Self {
        Self(skipped)
    }

        pub const fn add(&mut self, count: u32) {
        self.0 = self.0.saturating_add(count);
    }
}

#[derive(Debug, Clone)]
pub enum ActHoldPhase {
        Timed {
                remaining: Timer,
    },
                            AwaitingImpact {
                seen_busy: FxSeenBusy,
                cap:       Timer,
                        beat:      Duration,
    },
}

#[derive(Debug, Clone)]
pub struct ActHold {
        phase: ActHoldPhase,
}

impl ActHold {
        #[must_use]
    pub fn timed(seconds: f32) -> Self {
        Self {
            phase: ActHoldPhase::Timed {
                remaining: Timer::from_seconds(seconds.max(0.0), TimerMode::Once),
            },
        }
    }

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

        #[must_use]
    pub const fn phase(&self) -> &ActHoldPhase {
        &self.phase
    }

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

#[derive(Resource, Debug, Default)]
pub struct PlaybackCursor {
            shown:   ActSeq,
        holding: Option<ActHold>,
        skipped: SkippedActs,
}

impl PlaybackCursor {
        #[must_use]
    pub const fn shown(&self) -> ActSeq {
        self.shown
    }

        #[must_use]
    pub const fn is_holding(&self) -> bool {
        self.holding.is_some()
    }

        #[must_use]
    pub const fn hold(&self) -> Option<&ActHold> {
        self.holding.as_ref()
    }

        #[must_use]
    pub const fn skipped(&self) -> SkippedActs {
        self.skipped
    }

        pub fn reset(&mut self) {
        self.shown = ActSeq::START;
        self.holding = None;
        self.skipped = SkippedActs::default();
    }

        pub const fn hold_for(&mut self, hold: ActHold) {
        self.holding = Some(hold);
    }

            pub fn tick_hold(&mut self, delta: Duration, pipeline_busy: bool) -> bool {
        match self.holding.take() {
            None => true,
            Some(hold) => {
                self.holding = hold.advanced(delta, pipeline_busy);
                self.holding.is_none()
            }
        }
    }

        pub const fn advance_past_shown(&mut self) {
        self.shown = self.shown.next();
    }

        pub fn jump_to(&mut self, oldest: ActSeq) {
        self.skipped
            .add(u32::try_from(oldest.distance_from(self.shown)).unwrap_or(u32::MAX));
        self.shown = oldest;
    }
}
