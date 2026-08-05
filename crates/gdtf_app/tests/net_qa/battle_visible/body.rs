//! The reply shapes these cases decode, and the battles they read them from.

use gdtf_app::qa_wire::{
    inspect::InspectShownNet,
    roster::{FactionNet, GangerCardNet},
    visible::{VisibleCoverNet, VisibleDoorNet, VisibleGangerNet},
};
use gdtf_battle_sim::openable::OpenState;
use gdtf_qa_protocol::{command::RunOptions, message::QaResponse};
use serde::Deserialize;

use crate::{
    battle_setup::{
        ExpectedEnemy, SpawnedDoor, Standing, battle_with_a_door, battle_with_an_enemy,
    },
    command_exchange::{
        BATTLE_ROSTER, BATTLE_TURN, BATTLE_VISIBLE, exchange_expected, ran_body, run,
    },
    socket_support::TestError,
};

#[derive(Debug, Deserialize)]
pub(super) struct VisibleBody {
    pub(super) enemies: Vec<VisibleGangerNet>,
    pub(super) doors:   Vec<VisibleDoorNet>,
    pub(super) cover:   Vec<VisibleCoverNet>,
}

#[derive(Debug, Deserialize)]
pub(super) struct InspectBody {
    pub(super) shown: InspectShownNet,
}

#[derive(Debug, Deserialize)]
pub(super) struct RosterBody {
    pub(super) gangers: Vec<GangerCardNet>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TurnBody {
    pub(super) player: Option<FactionNet>,
}

/// What the three reads say about one battle holding a squad-visible enemy.
pub(super) struct LitArea {
    pub(super) visible: VisibleBody,
    pub(super) roster:  RosterBody,
    pub(super) turn:    TurnBody,
    pub(super) enemy:   ExpectedEnemy,
}

pub(super) fn decode<T: serde::de::DeserializeOwned>(
    name: &'static str,
    reply: Option<QaResponse>,
) -> Result<T, TestError> {
    let Some(reply) = reply else {
        return Err(format!("`{name}` produced no reply").into());
    };
    let body = ran_body(name, reply)?;
    ron::de::from_str::<T>(&body)
        .map_err(|fault| format!("`{name}`'s body must decode: {fault} — {body}").into())
}

/// Read the lit area, the roster and the turn from one battle holding a squad-visible enemy.
pub(super) fn lit_area() -> Result<LitArea, TestError> {
    let (replies, enemy) = exchange_expected(
        || battle_with_an_enemy(Standing::Lit),
        |_enemy| {
            vec![
                run(BATTLE_VISIBLE, "()", RunOptions::default()),
                run(BATTLE_ROSTER, "()", RunOptions::default()),
                run(BATTLE_TURN, "()", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let visible = decode::<VisibleBody>(BATTLE_VISIBLE, replies.next())?;
    let roster = decode::<RosterBody>(BATTLE_ROSTER, replies.next())?;
    let turn = decode::<TurnBody>(BATTLE_TURN, replies.next())?;
    Ok(LitArea {
        visible,
        roster,
        turn,
        enemy,
    })
}

/// Ask for the lit area in a battle where one enemy has been stood at a known cell.
pub(super) fn visible_with_an_enemy(
    standing: Standing,
) -> Result<(VisibleBody, ExpectedEnemy), TestError> {
    let (replies, enemy) = exchange_expected(
        move || battle_with_an_enemy(standing),
        |_enemy| vec![run(BATTLE_VISIBLE, "()", RunOptions::default())],
    )?;
    let visible = decode::<VisibleBody>(BATTLE_VISIBLE, replies.into_iter().next())?;
    Ok((visible, enemy))
}

/// Ask for the lit area in a battle holding one door in `state`, standing `standing`.
pub(super) fn visible_with_a_door(
    standing: Standing,
    state: OpenState,
) -> Result<(VisibleBody, SpawnedDoor), TestError> {
    let (replies, door) = exchange_expected(
        move || battle_with_a_door(standing, state),
        |_door| vec![run(BATTLE_VISIBLE, "()", RunOptions::default())],
    )?;
    let visible = decode::<VisibleBody>(BATTLE_VISIBLE, replies.into_iter().next())?;
    Ok((visible, door))
}
