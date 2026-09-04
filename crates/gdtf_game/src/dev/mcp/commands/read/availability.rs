//! Refusals for the availability words the battle reads and the acts use.

use cobalt_mcp_protocol::command::{CommandAvailability, RefusalNote, UnavailableCode};

use crate::dev::mcp::facts::{
    BattleActivity, BattleScreen, GameFacts, PlaybackCatchUp, PresenterReadiness, TurnOwner,
};

/// Available while the battle screen is up, at any battlescape phase.
pub(in crate::dev::mcp) const fn on_the_battle_screen(facts: GameFacts) -> CommandAvailability {
    match facts.battle_screen() {
        BattleScreen::Open => CommandAvailability::Available,
        BattleScreen::Closed => refuse("this reads the battle screen, and the game is not on it"),
    }
}

/// Available only in the battle's running phase.
pub(in crate::dev::mcp) const fn battle_is_running(facts: GameFacts) -> CommandAvailability {
    match facts.battle_activity() {
        BattleActivity::Running => CommandAvailability::Available,
        BattleActivity::NotRunning => refuse("this needs a battle in its running phase"),
    }
}

/// Available only when a running battle also has its sim state loaded.
pub(in crate::dev::mcp) const fn presenter_is_ready(facts: GameFacts) -> CommandAvailability {
    ready_or(
        facts,
        "this reads what the battle panels are showing, and no running battle is loaded for them \
         to show",
    )
}

/// Available only while a battle is live: running phase, sim state loaded.
pub(in crate::dev::mcp) const fn battle_is_live(facts: GameFacts) -> CommandAvailability {
    ready_or(
        facts,
        "this drives the view or the battle controls, and needs a running battle with its sim \
         state loaded",
    )
}

/// Available only in a running battle whose screen has caught up with the act log.
pub(in crate::dev::mcp) const fn running_and_caught(facts: GameFacts) -> CommandAvailability {
    match facts.battle_activity() {
        BattleActivity::NotRunning => acts_in_a_running_battle(facts),
        BattleActivity::Running => screen_has_caught_up(facts),
    }
}

/// Available only in a running battle, which is the only place an act is taken.
pub(in crate::dev::mcp) const fn acts_in_a_running_battle(facts: GameFacts) -> CommandAvailability {
    match facts.battle_activity() {
        BattleActivity::Running => CommandAvailability::Available,
        BattleActivity::NotRunning => {
            refuse("this acts in a running battle, and no battle is running")
        }
    }
}

/// Available only while the screen has caught up with the act log.
pub(in crate::dev::mcp) const fn screen_has_caught_up(facts: GameFacts) -> CommandAvailability {
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

/// Available only while the faction the player commands is the one acting.
pub(in crate::dev::mcp) const fn turn_is_the_players(facts: GameFacts) -> CommandAvailability {
    match facts.turn_owner() {
        TurnOwner::Player => CommandAvailability::Available,
        TurnOwner::OtherFaction => refuse(
            "the turn belongs to another faction, and no player path ends someone else's turn; \
             wait on `TurnChanged` to get past it",
        ),
    }
}

const fn ready_or(facts: GameFacts, note: &'static str) -> CommandAvailability {
    match facts.presenter_readiness() {
        PresenterReadiness::Ready => CommandAvailability::Available,
        PresenterReadiness::NotReady => refuse(note),
    }
}

const fn refuse(note: &'static str) -> CommandAvailability {
    CommandAvailability::Unavailable {
        code: UnavailableCode::WrongState,
        note: RefusalNote::from_static(note),
    }
}
