use cobalt_mcp_protocol::{command::RunOptions, message::McpResponse};
use gdtf_battle_sim::{ganger::Tu, weapon::ModeKind};
use gdtf_game::qa_wire::{
    cell::CellLevelNet,
    sight::{CanEngageNet, CanSeeNet, SightlineNet},
    token::GangerToken,
};
use serde::Deserialize;

use super::{
    battle_reads::{a_player_ganger, an_unreachable_cell, cell_argument},
    battle_setup::{MagazineLoad, battle_with_a_shooter_facing_north, battle_with_a_two_mode_gun},
    command_exchange::{
        BATTLE_SIGHTLINE, assert_refused_off_the_battle_screen, exchange_expected,
        exchange_planned, ran_body, run,
    },
    socket_support::{TestError, TestResult, battle_app_listening, game_app_listening},
};

/// More time units than any turn-and-fire ever costs.
const AMPLE_TU: Tu = Tu::new(u8::MAX);

/// The reply shape `battle.sightline` publishes, decoded the way a client decodes it.
#[derive(Debug, Deserialize)]
pub(crate) struct SightlineBody {
    pub(crate) at:        CellLevelNet,
    pub(crate) shooter:   Option<GangerToken>,
    pub(crate) sightline: SightlineNet,
}

/// The body of one reply, or a failure naming what came back instead.
pub(crate) fn sightline_body(reply: Option<McpResponse>) -> Result<SightlineBody, TestError> {
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
fn engaging_the_cell_behind(tu: Tu, load: MagazineLoad) -> Result<SightlineBody, TestError> {
    let (replies, _posed) = exchange_expected(
        move || battle_with_a_shooter_facing_north(tu, load),
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

/// The engage half of one answer, or a failure when the reply carried no shooter.
pub(crate) fn engage_answer(sightline: &SightlineBody) -> Result<CanEngageNet, TestError> {
    match sightline.sightline {
        SightlineNet::Answered { can_engage, .. } => Ok(can_engage),
        SightlineNet::NoShooter => Err(format!(
            "a shooter must be selected for the answer to carry both halves: {sightline:?}"
        )
        .into()),
    }
}

#[test]
fn a_shooter_with_time_to_turn_can_engage_the_cell_behind_it() -> TestResult {
    let sightline = engaging_the_cell_behind(AMPLE_TU, MagazineLoad::Loaded)?;

    let SightlineNet::Answered { can_engage, .. } = sightline.sightline else {
        unreachable!("with a shooter selected the answer is not NoShooter: {sightline:?}");
    };
    assert_eq!(
        can_engage,
        CanEngageNet::new(true),
        "the cell sits outside the firing arc, so the sim prices a turn first — and this \
         shooter can afford it with a loaded gun: {sightline:?}",
    );
    Ok(())
}

#[test]
fn a_shooter_whose_magazine_is_dry_cannot_engage_a_cell_it_otherwise_could() -> TestResult {
    let loaded = engaging_the_cell_behind(AMPLE_TU, MagazineLoad::Loaded)?;
    assert_eq!(
        engage_answer(&loaded)?,
        CanEngageNet::new(true),
        "the same shooter with rounds in the gun can engage the cell, so the dry answer below \
         cannot come from a fixture that never worked: {loaded:?}",
    );

    let dry = engaging_the_cell_behind(AMPLE_TU, MagazineLoad::Empty)?;
    assert_eq!(
        engage_answer(&dry)?,
        CanEngageNet::new(false),
        "an empty magazine is a shot the sim refuses, so engaging must be false even though the \
         arc and the pool still allow it: {dry:?}",
    );
    Ok(())
}

#[test]
fn a_shooter_with_no_time_left_cannot_engage_the_cell_behind_it() -> TestResult {
    let sightline = engaging_the_cell_behind(Tu::new(0), MagazineLoad::Loaded)?;

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

/// Ask about the cell behind a shooter whose two-mode gun is set to `chosen`.
fn engaging_the_cell_behind_a_gun_set_to(
    chosen: Option<ModeKind>,
) -> Result<SightlineBody, TestError> {
    let (replies, _posed) = exchange_expected(
        move || battle_with_a_two_mode_gun(chosen),
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
fn a_shooter_whose_gun_is_set_to_no_mode_is_priced_with_its_single() -> TestResult {
    let sightline = engaging_the_cell_behind_a_gun_set_to(None)?;

    assert_eq!(
        engage_answer(&sightline)?,
        CanEngageNet::new(true),
        "the fixture leaves the shooter holding exactly what turning and firing the gun's single \
         costs, and a gun nobody set a mode on fires that single: {sightline:?}",
    );
    Ok(())
}

#[test]
fn a_shooter_set_to_a_mode_it_cannot_pay_for_cannot_engage_the_cell_behind_it() -> TestResult {
    let sightline = engaging_the_cell_behind_a_gun_set_to(Some(ModeKind::Burst))?;

    assert_eq!(
        engage_answer(&sightline)?,
        CanEngageNet::new(false),
        "the same shooter, pose and pool with burst picked on the gun cannot pay for the shot, so \
         an engage answer of true means the read priced a mode the shooter is not on: \
         {sightline:?}",
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
