//! Refusals for the availability words the battle reads and the acts use.

use gdtf_qa_protocol::command::{CommandAvailability, RefusalNote, UnavailableCode};

use crate::dev::net_qa::facts::{
    BattleActivity, BattleScreen, GameFacts, PlaybackCatchUp, PresenterReadiness,
};

/// Available while the battle screen is up, at any battlescape phase.
pub(in crate::dev::net_qa) const fn on_the_battle_screen(facts: GameFacts) -> CommandAvailability {
    match facts.battle_screen() {
        BattleScreen::Open => CommandAvailability::Available,
        BattleScreen::Closed => refuse("this reads the battle screen, and the game is not on it"),
    }
}

/// Available only in the battle's running phase.
pub(in crate::dev::net_qa) const fn battle_is_running(facts: GameFacts) -> CommandAvailability {
    match facts.battle_activity() {
        BattleActivity::Running => CommandAvailability::Available,
        BattleActivity::NotRunning => refuse("this needs a battle in its running phase"),
    }
}

/// Available only when a running battle also has its sim state loaded.
pub(in crate::dev::net_qa) const fn presenter_is_ready(facts: GameFacts) -> CommandAvailability {
    match facts.presenter_readiness() {
        PresenterReadiness::Ready => CommandAvailability::Available,
        PresenterReadiness::NotReady => refuse(
            "this reads what the battle panels are showing, and no running battle is loaded for \
             them to show",
        ),
    }
}

/// Available only in a running battle whose screen has caught up with the act log.
pub(in crate::dev::net_qa) const fn running_and_caught(facts: GameFacts) -> CommandAvailability {
    match facts.battle_activity() {
        BattleActivity::NotRunning => {
            return refuse("this acts in a running battle, and no battle is running");
        }
        BattleActivity::Running => {}
    }
    match facts.catch_up() {
        PlaybackCatchUp::CaughtUp => CommandAvailability::Available,
        PlaybackCatchUp::Behind => CommandAvailability::Unavailable {
            code: UnavailableCode::Replaying,
            note: RefusalNote::from_static(
                "the screen is still playing the act log back, and an act may not be taken until \
                 it has caught up",
            ),
        },
    }
}

const fn refuse(note: &'static str) -> CommandAvailability {
    CommandAvailability::Unavailable {
        code: UnavailableCode::WrongState,
        note: RefusalNote::from_static(note),
    }
}
