//! GTW-762 clause 6 (the squad-fog shadow): the presenter's [`ShownSquadVisibility`] stays
//! FROZEN while closed-gate playback runs and PROMOTES the instant the cursor catches up.
//!
//! Mirrors the bare-cursor harness: a hand-built cursor + act log, one `promote_shown_fog`
//! run at a time, NO wall-clock waits. The gate is closed by growing the log (the head moves
//! past the still-at-start cursor) and opened by advancing the cursor to the head.

use bevy::{ecs::system::RunSystemOnce, platform::collections::HashSet};
use gdtf_battle_presenter::{PlaybackCursor, ShownSquadVisibility, promote_shown_fog};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, RecordedAct},
    ganger::Faction,
    metric::CellLevel,
    visibility::SquadVisibility,
};

use super::harness::{ground, playback_app};

/// A squad fog in which `cell` is VISIBLE (and EXPLORED — the accrual invariant).
fn fog_with(cell: CellLevel) -> SquadVisibility {
    let mut set = HashSet::new();
    set.insert(cell);
    SquadVisibility::new(set.clone(), set)
}

/// Grow the act log so a cursor still at the start is no longer caught up (the gate closes).
fn close_gate(app: &mut bevy::app::App) {
    let entity = app.world_mut().spawn_empty().id();
    let Some(mut log) = app.world_mut().get_resource_mut::<ActLog>() else {
        unreachable!("the fixture inserts an act log");
    };
    log.append(RecordedAct::new(
        entity,
        ActProvenance::Clock,
        ActDeed::TurnBegan {
            now_active: Faction::new(0),
        },
    ));
}

/// Advance the cursor to the log head — the cursor catches up, so the gate opens.
fn catch_up(app: &mut bevy::app::App) {
    let head = {
        let Some(log) = app.world().get_resource::<ActLog>() else {
            unreachable!("the fixture inserts an act log");
        };
        log.head()
    };
    let Some(mut cursor) = app.world_mut().get_resource_mut::<PlaybackCursor>() else {
        unreachable!("register_playback inits the cursor");
    };
    while cursor.shown() < head {
        cursor.advance_past_shown();
    }
}

/// Run the squad-fog promote once.
fn run_fog_promote(app: &mut bevy::app::App) {
    let ran = app.world_mut().run_system_once(promote_shown_fog);
    assert!(ran.is_ok(), "the squad-fog promote must run in the fixture");
}

/// Whether the shadow currently SHOWS `cell` as squad-VISIBLE.
fn shadow_shows(app: &bevy::app::App, cell: CellLevel) -> bool {
    *app.world()
        .resource::<ShownSquadVisibility>()
        .visibility()
        .is_cell_visible(&cell)
}

/// The squad-fog shadow FREEZES while closed-gate playback runs and PROMOTES the instant the
/// cursor catches up — so `present_fog` (which reads the shadow) never reveals a cell the sim
/// has already unfogged but the view has not yet played.
#[test]
fn squad_fog_shadow_freezes_then_promotes() {
    let mut app = playback_app();
    app.init_resource::<ShownSquadVisibility>();
    let cell = ground(3, 3);

    // State A: the live fog shows `cell`. Gate open (empty log) -> promote copies A.
    app.insert_resource(fog_with(cell));
    run_fog_promote(&mut app);
    assert!(
        shadow_shows(&app, cell),
        "the shadow promotes the live fog while caught up (state A: cell VISIBLE)",
    );

    // The sim reaches state B (the cell is no longer visible) AND the gate closes.
    app.insert_resource(SquadVisibility::default());
    close_gate(&mut app);
    run_fog_promote(&mut app);
    assert!(
        shadow_shows(&app, cell),
        "the shadow stays FROZEN at state A while the gate is shut, even though live is B",
    );

    // The cursor catches up -> the gate opens -> the shadow promotes to B.
    catch_up(&mut app);
    run_fog_promote(&mut app);
    assert!(
        !shadow_shows(&app, cell),
        "the shadow PROMOTES to state B the instant the cursor catches up",
    );
}
