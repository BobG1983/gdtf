//! Running a fixture's battle on until the screen has played back everything it wrote.

use bevy::app::App;
use gdtf_battle_presenter::PlaybackCursor;
use gdtf_battle_sim::act_log::{ActLog, ActSeq};

use crate::socket_support::TestError;

/// Frames a fixture may spend on playback before it gives up and fails the case.
const CATCH_UP_BUDGET: u32 = 4096;

/// Whether the screen has played the act log to its head and is not holding on an entry.
pub(crate) fn the_screen_has_caught_up(app: &App) -> bool {
    let (Some(log), Some(cursor)) = (
        app.world().get_resource::<ActLog>(),
        app.world().get_resource::<PlaybackCursor>(),
    ) else {
        return false;
    };
    !cursor.is_holding() && cursor.shown() >= log.head()
}

/// Run frames until the act log has stopped growing and the screen has played all of it.
pub(crate) fn let_the_screen_catch_up(app: &mut App) -> Result<(), TestError> {
    let mut logged: Option<ActSeq> = None;
    for _ in 0..CATCH_UP_BUDGET {
        app.update();
        let head = head_of(app);
        if logged == head && the_screen_has_caught_up(app) {
            return Ok(());
        }
        logged = head;
    }
    Err(format!(
        "the screen never caught up with a settled act log inside {CATCH_UP_BUDGET} frames; the \
         log head was {:?} and the screen had shown {:?}",
        head_of(app),
        app.world()
            .get_resource::<PlaybackCursor>()
            .map(PlaybackCursor::shown),
    )
    .into())
}

fn head_of(app: &App) -> Option<ActSeq> {
    app.world().get_resource::<ActLog>().map(ActLog::head)
}
