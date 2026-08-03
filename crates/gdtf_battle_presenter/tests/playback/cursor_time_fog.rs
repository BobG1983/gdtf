use bevy::{ecs::system::RunSystemOnce, platform::collections::HashSet};
use gdtf_battle_presenter::{PlaybackCursor, ShownSquadVisibility, promote_shown_fog};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, RecordedAct},
    ganger::Faction,
    metric::CellLevel,
    visibility::SquadVisibility,
};

use super::harness::{ground, playback_app};

fn fog_with(cell: CellLevel) -> SquadVisibility {
    let mut set = HashSet::new();
    set.insert(cell);
    SquadVisibility::new(set.clone(), set)
}

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

fn run_fog_promote(app: &mut bevy::app::App) {
    let ran = app.world_mut().run_system_once(promote_shown_fog);
    assert!(ran.is_ok(), "the squad-fog promote must run in the fixture");
}

fn shadow_shows(app: &bevy::app::App, cell: CellLevel) -> bool {
    *app.world()
        .resource::<ShownSquadVisibility>()
        .visibility()
        .is_cell_visible(&cell)
}

#[test]
fn squad_fog_shadow_freezes_then_promotes() {
    let mut app = playback_app();
    app.init_resource::<ShownSquadVisibility>();
    let cell = ground(3, 3);

    app.insert_resource(fog_with(cell));
    run_fog_promote(&mut app);
    assert!(
        shadow_shows(&app, cell),
        "the shadow promotes the live fog while caught up (state A: cell VISIBLE)",
    );

    app.insert_resource(SquadVisibility::default());
    close_gate(&mut app);
    run_fog_promote(&mut app);
    assert!(
        shadow_shows(&app, cell),
        "the shadow stays FROZEN at state A while the gate is shut, even though live is B",
    );

    catch_up(&mut app);
    run_fog_promote(&mut app);
    assert!(
        !shadow_shows(&app, cell),
        "the shadow PROMOTES to state B the instant the cursor catches up",
    );
}
