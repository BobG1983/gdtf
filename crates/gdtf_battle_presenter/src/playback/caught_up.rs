//! [`playback_caught_up`] and [`PlaybackGate`] — the ONE catch-up predicate the global
//! input gate is built on (GTW-727 C19 / C22 / C35).

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::act_log::ActLog;

use super::cursor::PlaybackCursor;

/// The catch-up reads, as a [`SystemParam`] — for the two drains, which cannot use a run
/// condition (see below) and must ask the same question INSIDE their body.
///
/// One predicate, two call shapes: [`playback_caught_up`] is this same test as a run
/// condition. Keeping the logic in one place is what stops the gate a system is registered
/// under and the gate it applies per-intent from ever drifting apart.
#[derive(SystemParam)]
pub struct PlaybackGate<'w> {
    /// The presenter's cursor — absent wherever there is no presenter.
    cursor: Option<Res<'w, PlaybackCursor>>,
    /// The battle's act log — absent outside a live battle.
    log:    Option<Res<'w, ActLog>>,
}

impl PlaybackGate<'_> {
    /// Whether the player may act right now — i.e. whether the screen shows everything the
    /// sim has done.
    ///
    /// Fails OPEN when either resource is absent, for the reason spelled out on
    /// [`playback_caught_up`].
    #[must_use]
    pub fn is_open(&self) -> bool {
        let (Some(cursor), Some(log)) = (self.cursor.as_deref(), self.log.as_deref()) else {
            return true;
        };
        !cursor.is_holding() && cursor.shown() >= log.head()
    }
}

/// Whether the presenter has SHOWN everything the sim has done — the run condition the
/// global input gate keys on.
///
/// `caught_up == no hold in progress && nothing left unshown`.
///
/// ## It reads no entity population, deliberately
///
/// A tempting extra conjunct is "and no projectile is still in flight". It must not be
/// added. Projectile flight time is a hot-reloadable tuning value, so an authored velocity
/// of zero — or any single leaked bolt — would make that conjunct permanently false, and
/// with input gated shut and no in-battle quit key the only way out of the resulting
/// soft-lock is killing the process. The cursor's own hold already bounds the wait for a
/// bolt in wall-clock time; this predicate reads nothing but the cursor and the log.
///
/// ## It FAILS OPEN
///
/// With no cursor or no log, this returns `true` — input flows normally. That deliberately
/// inverts this codebase's usual fail-CLOSED convention for absent resources, and the
/// inversion is the point: a headless app, an autobattle run, or any harness without the
/// top-down renderer has no cursor at all, so a fail-closed gate there would be a permanent
/// deadlock rather than a safe default. Gating is a presenter affordance; where there is no
/// presenter there is nothing to wait for.
///
/// ## It is evaluated one frame before the cursor moves, and that is safe
///
/// Run conditions on the input band are evaluated before the sim runs, while the cursor
/// advances after it. So this answer can be one frame stale — always in the SAFE direction:
/// a log that grows this frame closes the gate next frame, and the gate never opens early.
#[must_use]
pub fn playback_caught_up(cursor: Option<Res<PlaybackCursor>>, log: Option<Res<ActLog>>) -> bool {
    let (Some(cursor), Some(log)) = (cursor, log) else {
        return true;
    };
    !cursor.is_holding() && cursor.shown() >= log.head()
}
