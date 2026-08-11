//! Stance over a real socket: the stance the command names is the stance the roster draws.

use gdtf_app::qa_wire::{act::ActReply, act_payload::StanceNet, refusal::StanceRefusalNet};
use gdtf_qa_protocol::command::RunOptions;

use super::{
    act_support::{
        RosterBody, accepted, answered, assert_caught_up, card_of, caught_up, complete, decode,
        next, selected,
    },
    command_exchange::{
        ACT_SELECT_NEXT, ACT_SET_STANCE, BATTLE_ROSTER, WAIT, assert_refused_off_the_battle_screen,
        exchange_all, run,
    },
    socket_support::{TestResult, battle_app_listening, game_app_listening},
};

#[test]
fn setting_a_stance_puts_that_ganger_prone_on_the_roster() -> TestResult {
    let replies = exchange_all(
        battle_app_listening,
        vec![
            run(ACT_SELECT_NEXT, "()", RunOptions::default()),
            run(ACT_SET_STANCE, "(stance:Prone)", RunOptions::default()),
            run(BATTLE_ROSTER, "()", RunOptions::default()),
        ],
    )?;
    let mut replies = replies.into_iter();
    let shooter = selected(ACT_SELECT_NEXT, next(ACT_SELECT_NEXT, &mut replies)?)?;
    let posture = accepted(ACT_SET_STANCE, next(ACT_SET_STANCE, &mut replies)?)?;
    let roster = decode::<RosterBody>(BATTLE_ROSTER, next(BATTLE_ROSTER, &mut replies)?)?;

    let Some(shooter) = shooter else {
        unreachable!("a posture needs a selected shooter to apply to");
    };
    let Some(card) = card_of(&roster, shooter) else {
        unreachable!("the selected shooter is on the roster: {roster:?}");
    };
    assert_eq!(
        card.stance,
        StanceNet::Prone,
        "the named stance is absolute, so the card the panels draw must read Prone: {card:?}",
    );
    assert_eq!(
        complete(posture).map(|done| *done),
        Some(true),
        "dropping prone takes no walking, so the act reports itself finished in its own frame",
    );
    Ok(())
}

#[test]
fn setting_the_same_stance_twice_leaves_the_same_stance() -> TestResult {
    let replies = exchange_all(
        battle_app_listening,
        vec![
            run(ACT_SELECT_NEXT, "()", RunOptions::default()),
            run(ACT_SET_STANCE, "(stance:Crouching)", RunOptions::default()),
            caught_up(),
            run(ACT_SET_STANCE, "(stance:Crouching)", RunOptions::default()),
            run(BATTLE_ROSTER, "()", RunOptions::default()),
        ],
    )?;
    let mut replies = replies.into_iter();
    let shooter = selected(ACT_SELECT_NEXT, next(ACT_SELECT_NEXT, &mut replies)?)?;
    let _once = accepted(ACT_SET_STANCE, next(ACT_SET_STANCE, &mut replies)?)?;
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let twice = answered(ACT_SET_STANCE, next(ACT_SET_STANCE, &mut replies)?)?;
    let roster = decode::<RosterBody>(BATTLE_ROSTER, next(BATTLE_ROSTER, &mut replies)?)?;

    let Some(shooter) = shooter else {
        unreachable!("a posture needs a selected shooter to apply to");
    };
    let Some(card) = card_of(&roster, shooter) else {
        unreachable!("the selected shooter is on the roster: {roster:?}");
    };
    assert_eq!(
        card.stance,
        StanceNet::Crouching,
        "an absolute stance is idempotent, unlike the cycle keybind: {card:?}",
    );
    assert_eq!(
        twice,
        ActReply::StanceRefused {
            reason: StanceRefusalNet::AlreadyHeld,
        },
        "the sim will not change a stance the actor already holds, so the second call answers \
         that reason rather than a window that claims it did something",
    );
    Ok(())
}

#[test]
fn the_stance_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, ACT_SET_STANCE, "(stance:Prone)")?;
    Ok(())
}
