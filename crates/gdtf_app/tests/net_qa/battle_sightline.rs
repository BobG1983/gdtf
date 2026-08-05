use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    sight::{CanEngageNet, CanSeeNet, SightlineNet},
    token::GangerToken,
};
use gdtf_battle_sim::ganger::Tu;
use gdtf_qa_protocol::{command::RunOptions, message::QaResponse};
use serde::Deserialize;

use super::{
    battle_reads::{a_player_ganger, an_unreachable_cell, cell_argument},
    battle_setup::battle_with_a_shooter_facing_north,
    command_exchange::{
        BATTLE_SIGHTLINE, assert_refused_off_the_battle_screen, exchange_expected,
        exchange_planned, ran_body, run,
    },
    socket_support::{TestError, TestResult, battle_app_listening, game_app_listening},
};

/// More time units than any turn-and-fire ever costs.
const AMPLE_TU: Tu = Tu::new(u8::MAX);

#[derive(Debug, Deserialize)]
struct SightlineBody {
    at:        CellLevelNet,
    shooter:   Option<GangerToken>,
    sightline: SightlineNet,
}

fn sightline_body(reply: Option<QaResponse>) -> Result<SightlineBody, TestError> {
    let Some(reply) = reply else {
        return Err("battle.sightline produced no reply".into());
    };
    let body = ran_body(BATTLE_SIGHTLINE, reply)?;
    ron::de::from_str::<SightlineBody>(&body)
        .map_err(|fault| format!("the sightline body must decode: {fault} — {body}").into())
}

#[test]
fn a_cell_a_squadmate_stands_on_is_seen() -> TestResult {
    let replies = exchange_planned(battle_app_listening, |app| {
        let argument =
            a_player_ganger(app).map_or_else(|| "()".to_owned(), |(_, at)| cell_argument(at));
        vec![run(BATTLE_SIGHTLINE, &argument, RunOptions::default())]
    })?;
    let sightline = sightline_body(replies.into_iter().next())?;

    assert!(
        sightline.shooter.is_some(),
        "a running battle has a shooter selected, so the answer is not NoShooter: \
         {sightline:?}",
    );
    let SightlineNet::Answered { can_see, .. } = sightline.sightline else {
        unreachable!("with a shooter selected the answer is not NoShooter: {sightline:?}");
    };
    assert_eq!(
        can_see,
        CanSeeNet::new(true),
        "own-squad members are always squad-visible, which is the sim's own rule: \
         {sightline:?}",
    );
    Ok(())
}

#[test]
fn a_cell_off_the_map_is_not_seen() -> TestResult {
    let replies = exchange_planned(battle_app_listening, |_app| {
        vec![run(
            BATTLE_SIGHTLINE,
            &cell_argument(an_unreachable_cell()),
            RunOptions::default(),
        )]
    })?;
    let sightline = sightline_body(replies.into_iter().next())?;

    let SightlineNet::Answered { can_see, .. } = sightline.sightline else {
        unreachable!("with a shooter selected the answer is not NoShooter: {sightline:?}");
    };
    assert_eq!(
        can_see,
        CanSeeNet::new(false),
        "nothing off the map is in the squad's field of view, so the answer tracks the live \
         fog rather than a constant: {sightline:?}",
    );
    assert_eq!(
        sightline.at,
        an_unreachable_cell(),
        "the reply echoes the cell it was asked about: {sightline:?}",
    );
    Ok(())
}

/// Ask about the cell directly behind a north-facing shooter holding `tu` time units.
fn engaging_the_cell_behind(tu: Tu) -> Result<SightlineBody, TestError> {
    let (replies, _posed) = exchange_expected(
        move || battle_with_a_shooter_facing_north(tu),
        |posed| {
            vec![run(
                BATTLE_SIGHTLINE,
                &cell_argument(posed.behind),
                RunOptions::default(),
            )]
        },
    )?;
    sightline_body(replies.into_iter().next())
}

#[test]
fn a_shooter_with_time_to_turn_can_engage_the_cell_behind_it() -> TestResult {
    let sightline = engaging_the_cell_behind(AMPLE_TU)?;

    let SightlineNet::Answered { can_engage, .. } = sightline.sightline else {
        unreachable!("with a shooter selected the answer is not NoShooter: {sightline:?}");
    };
    assert_eq!(
        can_engage,
        CanEngageNet::new(true),
        "the cell sits outside the firing arc, so the sim prices a turn first — and this \
         shooter can afford it: {sightline:?}",
    );
    Ok(())
}

#[test]
fn a_shooter_with_no_time_left_cannot_engage_the_cell_behind_it() -> TestResult {
    let sightline = engaging_the_cell_behind(Tu::new(0))?;

    let SightlineNet::Answered { can_engage, .. } = sightline.sightline else {
        unreachable!("with a shooter selected the answer is not NoShooter: {sightline:?}");
    };
    assert_eq!(
        can_engage,
        CanEngageNet::new(false),
        "the same cell out of arc with nothing left to spend on turning is refused, so the \
         answer tracks the sim's arc gate rather than a constant: {sightline:?}",
    );
    Ok(())
}

#[test]
fn sightline_refuses_outside_a_running_battle() -> TestResult {
    assert_refused_off_the_battle_screen(
        game_app_listening,
        BATTLE_SIGHTLINE,
        &cell_argument(an_unreachable_cell()),
    )?;
    Ok(())
}
