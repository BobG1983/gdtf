//! Running a fixture's battle on until the screen has played back everything it wrote.

use bevy::{app::App, ecs::system::RunSystemOnce};
use gdtf_battle_presenter::{PlaybackCursor, playback_caught_up};
use gdtf_battle_sim::act_log::{ActLog, ActSeq};

/// Whether the game's own playback gate is open, with a battle that never started counting as no.
pub(crate) fn the_screen_has_caught_up(app: &mut App) -> bool {
    let world = app.world();
    if world.get_resource::<ActLog>().is_none() || world.get_resource::<PlaybackCursor>().is_none()
    {
        return false;
    }
    app.world_mut()
        .run_system_once(playback_caught_up)
        .unwrap_or(false)
}

/// Run frames until the act log has stopped growing and the screen has played all of it.
pub(crate) fn let_the_screen_catch_up(app: &mut App) {
    let mut logged: Option<ActSeq> = None;
    loop {
        app.update();
        let head = head_of(app);
        if logged == head && the_screen_has_caught_up(app) {
            return;
        }
        logged = head;
    }
}

fn head_of(app: &App) -> Option<ActSeq> {
    app.world().get_resource::<ActLog>().map(ActLog::head)
}
