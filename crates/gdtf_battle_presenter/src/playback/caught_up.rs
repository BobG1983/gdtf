//! Input gate: open only when playback has caught the act-log head.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::act_log::ActLog;

use super::cursor::PlaybackCursor;

/// System-param view of whether player input may proceed.
#[derive(SystemParam)]
pub struct PlaybackGate<'w> {
    cursor: Option<Res<'w, PlaybackCursor>>,
    log:    Option<Res<'w, ActLog>>,
}

impl PlaybackGate<'_> {
    /// `true` when not holding and shown past or at the log head.
    #[must_use]
    pub fn is_open(&self) -> bool {
        caught_up(self.cursor.as_deref(), self.log.as_deref())
    }
}

/// Standalone check used by `run_if` filters.
#[must_use]
pub fn playback_caught_up(cursor: Option<Res<PlaybackCursor>>, log: Option<Res<ActLog>>) -> bool {
    caught_up(cursor.as_deref(), log.as_deref())
}

// The one gate condition; open when either resource is missing.
fn caught_up(cursor: Option<&PlaybackCursor>, log: Option<&ActLog>) -> bool {
    let (Some(cursor), Some(log)) = (cursor, log) else {
        return true;
    };
    !cursor.is_holding() && cursor.shown() >= log.head()
}
