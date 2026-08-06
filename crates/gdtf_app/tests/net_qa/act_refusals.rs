//! An act the QA layer can turn down itself is refused, rather than reaching the sim.

use std::sync::mpsc::Receiver;

use bevy::{app::App, ecs::entity::Entity};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_presenter::playback::PlaybackCursor;
use gdtf_battle_sim::{
    act_log::ActLog,
    battle::PlayerFaction,
    ganger::{Faction, LifeState},
};
use gdtf_qa_protocol::{command::CommandOutcome, message::QaResponse};

use crate::{
    battle_fixture::{drive_into_battle_running, menu_app_with_net_qa, run_request, send},
    battle_reads::ganger_argument,
    command_exchange::{ACT_MOVE, ACT_RELOAD, ACT_SELECT, ACT_SET_AIMING, ACT_SET_STANCE},
};

/// Frames a claimed act is allowed to take to answer.
const SETTLE_FRAMES: u32 = 8;

/// Any cell will do: the act never reaches the sim, so the destination is never read.
const SOMEWHERE: &str = "(at:(cell:(x:1,y:1),level:0))";

/// Put every one of the player's gangers down, and hand back who was taken away.
fn down_the_players_gang(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let Some(player) = world.get_resource::<PlayerFaction>().map(|player| **player) else {
        unreachable!("a running battle names the gang the player commands");
    };
    let gang: Vec<Entity> = world
        .iter_entities()
        .filter_map(|entity| (entity.get::<Faction>() == Some(&player)).then_some(entity.id()))
        .collect();
    assert!(
        !gang.is_empty(),
        "the fixture must field a player gang for this case to take it away",
    );
    for entity in &gang {
        if let Ok(mut ganger) = world.get_entity_mut(*entity) {
            ganger.insert(LifeState::Downed);
        }
    }
    *world.resource_mut::<SelectedShooter>() = SelectedShooter::cleared();

    // Downing the gang wrote act-log lines; put the screen on the head so the gate is open.
    let head = world.resource::<ActLog>().head();
    let mut cursor = world.resource_mut::<PlaybackCursor>();
    cursor.reset();
    cursor.jump_to(head);
    gang
}

/// Step until the reply lands, then hand back its RON body.
fn body_of(app: &mut App, reply: &Receiver<QaResponse>, named: &str) -> String {
    for _ in 0..SETTLE_FRAMES {
        app.update();
        if let Ok(answer) = reply.try_recv() {
            let QaResponse::Outcome(CommandOutcome::Ran { reply, .. }) = answer else {
                unreachable!("`{named}` must answer rather than refuse admission: {answer:?}");
            };
            return reply.as_str().to_owned();
        }
    }
    unreachable!("`{named}` must answer inside {SETTLE_FRAMES} frames")
}

/// Drive a battle whose gang is all down, then run one act against it.
fn refuse_with_nobody_selected(named: &'static str, arguments: &str) {
    let (mut app, tx) = menu_app_with_net_qa();
    drive_into_battle_running(&mut app);
    let _gang = down_the_players_gang(&mut app);

    let reply = send(&tx, run_request(named, arguments));
    let body = body_of(&mut app, &reply, named);
    assert!(
        body.contains("Refused") && body.contains("NoShooter"),
        "`{named}` needs a selected shooter and there is none, so it must be refused NoShooter \
         without touching the sim; got {body}",
    );
    assert!(
        app.world().resource::<SelectedShooter>().is_none(),
        "`{named}` must not select anyone on its way to refusing",
    );
}

#[test]
fn selecting_a_ganger_that_is_down_is_refused_unknown_token() {
    let (mut app, tx) = menu_app_with_net_qa();
    drive_into_battle_running(&mut app);
    let gang = down_the_players_gang(&mut app);
    let Some(downed) = gang.first().copied() else {
        unreachable!("downing the gang reports who it took away");
    };

    let reply = send(&tx, run_request(ACT_SELECT, &ganger_argument(downed)));
    let body = body_of(&mut app, &reply, ACT_SELECT);
    assert!(
        body.contains("Refused") && body.contains("UnknownToken"),
        "the game never selects a ganger that is down, so its token must come back refused \
         UnknownToken rather than being accepted and quietly dropped; got {body}",
    );
    assert!(
        app.world().resource::<SelectedShooter>().is_none(),
        "a refused token must not select anyone",
    );
}

#[test]
fn reloading_with_nobody_selected_is_refused_no_shooter() {
    refuse_with_nobody_selected(ACT_RELOAD, "()");
}

#[test]
fn a_posture_with_nobody_selected_is_refused_no_shooter() {
    refuse_with_nobody_selected(ACT_SET_STANCE, "(stance:Prone)");
    refuse_with_nobody_selected(ACT_SET_AIMING, "(aim:true)");
}

#[test]
fn a_move_with_nobody_selected_is_refused_no_shooter() {
    refuse_with_nobody_selected(ACT_MOVE, SOMEWHERE);
}
