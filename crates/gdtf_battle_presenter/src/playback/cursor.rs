//! [`PlaybackCursor`] — HOW FAR through the act log the presenter has shown, and what it
//! is currently holding on (GTW-727 C15 / C19 / C21).

use std::time::Duration;

use bevy::prelude::*;
use gdtf_battle_sim::act_log::ActSeq;

/// Whether the FX pipeline has been observed BUSY since a hold began.
///
/// The trap this guard closes is real and already documented next door in the app's
/// end-of-battle gate: a projectile is spawned through a DEFERRED scene spawn, so it is
/// simply not queryable on the frame its message was played. An "is the pipeline idle?"
/// test taken in that gap reads idle and would release the hold immediately — before the
/// bolt it is waiting for even exists. Idle therefore only counts as DRAINED once busy has
/// been seen first.
///
/// A named flag (`no-bare-types.md`), never a bare `bool`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FxSeenBusy(bool);

impl FxSeenBusy {
    /// Build the guard from whether the pipeline has been seen busy yet.
    #[must_use]
    pub const fn new(seen: bool) -> Self {
        Self(seen)
    }
}

/// How many act-log entries the cursor has SKIPPED because they fell out of the ring
/// before it reached them (GTW-727 C21).
///
/// A diagnostic count. Skipping is a degradation, never a stall: the cursor resyncs the
/// drawn world to live sim state and jumps to the head, so nothing on screen is left stale
/// and the view is instantly current again. It only ever moves FORWARD, so it can never
/// render a false sequence by rewinding, and it never blocks.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkippedActs(u32);

impl SkippedActs {
    /// Build a skip count from its total.
    #[must_use]
    pub const fn new(skipped: u32) -> Self {
        Self(skipped)
    }

    /// Add `count` skipped entries (saturating — the counter never wraps).
    pub const fn add(&mut self, count: u32) {
        self.0 = self.0.saturating_add(count);
    }
}

/// What the cursor is waiting for before it may release the next entry.
///
/// **Every phase is bounded in wall-clock time.** No hold may depend on an unbounded
/// entity population: a release condition of the form "wait until no projectiles remain"
/// soft-locks the whole game if a bolt ever leaks or the hot-reloadable projectile velocity
/// is authored as zero — and with the input gate closed and no in-battle quit key, the only
/// way out would be killing the process.
#[derive(Debug, Clone)]
pub enum ActHoldPhase {
    /// A plain timed beat — release when the timer elapses.
    Timed {
        /// The remaining beat.
        remaining: Timer,
    },
    /// Waiting for a round's projectile to fly and land: release when the FX pipeline has
    /// been observed BUSY and has since drained back to idle, OR when `cap` elapses.
    ///
    /// This is the hold that fixes the reported defect. Damage numbers, injuries, floating
    /// text and death all sit BEHIND this phase in the log, so they cannot be drawn until
    /// the bolt that caused them has actually arrived.
    AwaitingImpact {
        /// The trap guard: idle does not count as drained until busy has been seen.
        seen_busy: FxSeenBusy,
        /// The hard backstop, for the case where no bolt ever spawns.
        cap:       Timer,
        /// The beat to serve AFTER the impact resolves, so successive rounds of a burst
        /// stay a readable distance apart rather than running together.
        beat:      Duration,
    },
}

/// One in-progress hold.
#[derive(Debug, Clone)]
pub struct ActHold {
    /// What the hold is waiting for.
    phase: ActHoldPhase,
}

impl ActHold {
    /// Build a plain timed beat of `seconds`.
    #[must_use]
    pub fn timed(seconds: f32) -> Self {
        Self {
            phase: ActHoldPhase::Timed {
                remaining: Timer::from_seconds(seconds.max(0.0), TimerMode::Once),
            },
        }
    }

    /// Build an impact wait capped at `cap_seconds`, followed by a `beat_seconds` beat.
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

    /// The hold's current phase (inspection / test read).
    #[must_use]
    pub const fn phase(&self) -> &ActHoldPhase {
        &self.phase
    }

    /// Advance this hold by `delta`, given whether the FX pipeline is busy RIGHT NOW.
    ///
    /// Returns the hold to keep waiting on, or [`None`] when the cursor may release the
    /// next entry. An [`AwaitingImpact`](ActHoldPhase::AwaitingImpact) that finishes
    /// waiting converts into its follow-on [`Timed`](ActHoldPhase::Timed) beat rather than
    /// clearing, so the round's own beat is served after the impact rather than instead of
    /// it.
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
                    // The bolt flew and landed (busy → idle), or the backstop fired
                    // because no bolt was ever spawned. Either way the wait is over; serve
                    // the round's beat next.
                    let beat = *beat;
                    self.phase = ActHoldPhase::Timed {
                        remaining: Timer::new(beat, TimerMode::Once),
                    };
                    return Some(self);
                }
                if cap.is_finished() {
                    // Busy has been seen but the bolt is still outstanding past the cap —
                    // a leaked or zero-velocity projectile. Give up waiting on it.
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

/// The presenter's PLAYBACK CURSOR — how far through the battle's act log the view has got.
///
/// Process-lifetime (`init_resource`'d by the renderer plugin), NOT battle-scoped: per-battle
/// reset is SELF-HEALING off the log's own absence, so no app state boundary has to own a
/// presenter resource. A battle that ends removes the log, the cursor sees `None` and
/// resets; a battle that starts inserts a fresh log at sequence zero and the cursor is
/// already there.
#[derive(Resource, Debug, Default)]
pub struct PlaybackCursor {
    /// The sequence number of the next entry to show — everything BELOW this has been
    /// shown, so `shown == log.head()` means fully caught up.
    shown:   ActSeq,
    /// The hold in progress, if any.
    holding: Option<ActHold>,
    /// How many entries have been skipped because they fell out of the ring.
    skipped: SkippedActs,
}

impl PlaybackCursor {
    /// The sequence number of the next entry to show.
    #[must_use]
    pub const fn shown(&self) -> ActSeq {
        self.shown
    }

    /// Whether the cursor is currently holding on an act.
    #[must_use]
    pub const fn is_holding(&self) -> bool {
        self.holding.is_some()
    }

    /// The hold in progress, if any (inspection / test read).
    #[must_use]
    pub const fn hold(&self) -> Option<&ActHold> {
        self.holding.as_ref()
    }

    /// How many entries have been skipped because they fell out of the ring.
    #[must_use]
    pub const fn skipped(&self) -> SkippedActs {
        self.skipped
    }

    /// Reset to the start of a fresh battle — no hold, nothing shown, no skips.
    pub fn reset(&mut self) {
        self.shown = ActSeq::START;
        self.holding = None;
        self.skipped = SkippedActs::default();
    }

    /// Begin `hold` after releasing an entry.
    pub const fn hold_for(&mut self, hold: ActHold) {
        self.holding = Some(hold);
    }

    /// Advance the current hold; returns `true` when the cursor is free to release an
    /// entry this frame.
    pub fn tick_hold(&mut self, delta: Duration, pipeline_busy: bool) -> bool {
        match self.holding.take() {
            None => true,
            Some(hold) => {
                self.holding = hold.advanced(delta, pipeline_busy);
                self.holding.is_none()
            }
        }
    }

    /// Mark the entry at [`shown`](Self::shown) as played and step past it.
    pub const fn advance_past_shown(&mut self) {
        self.shown = self.shown.next();
    }

    /// Jump forward to `oldest`, counting the entries skipped on the way.
    pub fn jump_to(&mut self, oldest: ActSeq) {
        self.skipped
            .add(u32::try_from(oldest.distance_from(self.shown)).unwrap_or(u32::MAX));
        self.shown = oldest;
    }
}
