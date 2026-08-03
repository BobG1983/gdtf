use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::act_log::ActLog;

use super::cursor::PlaybackCursor;

#[derive(SystemParam)]
pub struct PlaybackGate<'w> {
        cursor: Option<Res<'w, PlaybackCursor>>,
        log:    Option<Res<'w, ActLog>>,
}

impl PlaybackGate<'_> {
                        #[must_use]
    pub fn is_open(&self) -> bool {
        let (Some(cursor), Some(log)) = (self.cursor.as_deref(), self.log.as_deref()) else {
            return true;
        };
        !cursor.is_holding() && cursor.shown() >= log.head()
    }
}

#[must_use]
pub fn playback_caught_up(cursor: Option<Res<PlaybackCursor>>, log: Option<Res<ActLog>>) -> bool {
    let (Some(cursor), Some(log)) = (cursor, log) else {
        return true;
    };
    !cursor.is_holding() && cursor.shown() >= log.head()
}
