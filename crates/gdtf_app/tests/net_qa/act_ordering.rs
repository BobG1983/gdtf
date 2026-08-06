//! The act commands must claim before the act bus drains and settle after the sim records.

use bevy::ecs::schedule::IntoSystemSet;
use gdtf_app::test_support::ActCommandSystems;
use gdtf_battle_input::auto_select_first_player_ganger;
use gdtf_battle_sim::occupancy_sync::SimSystems;
use gdtf_qa_command::dispatch::QaCommandSystems;

use super::{
    schedule_support::{
        a_system_named, covers, members, ordered_before, set_node, update_schedule,
    },
    socket_support::{TestResult, battle_app_listening},
};

/// One command claims and settles, so each band holds the eleven acts plus `input.click_cell`.
const BANDED_ACTS: usize = 12;

#[test]
fn every_act_claim_runs_before_the_act_bus_drains() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let claims = members(graph, ActCommandSystems::Claim)?;
    assert_eq!(
        claims.len(),
        BANDED_ACTS,
        "every command that pushes onto the act bus must register its claim system in the band, \
         or the band's ordering does not apply to it",
    );
    let drain = a_system_named(update, "dispatch_act_intents")?;
    assert!(
        ordered_before(graph, set_node(graph, ActCommandSystems::Claim)?, drain),
        "the claim band must be ordered before dispatch_act_intents; without that edge an act \
         command shares InputSystems::Gather with the drain and its intent lands a frame late",
    );
    Ok(())
}

#[test]
fn every_command_claims_after_the_game_has_settled_its_selection() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let auto_select = a_system_named(update, "auto_select_first_player_ganger")?;
    let picks = set_node(graph, auto_select_first_player_ganger.into_system_set())?;
    assert!(
        covers(graph, picks, auto_select),
        "the app must register auto_select_first_player_ganger, or ordering against it orders \
         against nothing",
    );
    let claims = members(graph, QaCommandSystems::Claim)?;
    assert!(
        !claims.is_empty(),
        "the command set must register its claim systems, or this ordering means nothing",
    );
    for claim in claims {
        assert!(
            ordered_before(graph, picks, claim),
            "a command must claim after auto_select_first_player_ganger; sharing \
             InputSystems::Gather with it leaves the order open, so a command claimed in the \
             frame after a clear reads either the empty selection or the one the game re-picked",
        );
    }
    let act_claim = a_system_named(update, "claim_act_select_clear")?;
    assert!(
        ordered_before(graph, set_node(graph, QaCommandSystems::Claim)?, act_claim),
        "the act claim band must be ordered after the command claim band, which is what carries \
         the auto-select edge on to the acts",
    );
    Ok(())
}

#[test]
fn every_act_settle_runs_after_the_sim_records_the_frame() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let settles = members(graph, ActCommandSystems::Settle)?;
    assert_eq!(
        settles.len(),
        BANDED_ACTS,
        "every command that answers an act-log window must register its settle system in the band",
    );
    let recorder = a_system_named(update, "record_acts")?;
    assert!(
        members(graph, SimSystems::Record)?.contains(&recorder),
        "the act log's recorder must sit in SimSystems::Record, which is what the settle band \
         waits on",
    );
    for settle in settles {
        assert!(
            ordered_before(graph, set_node(graph, SimSystems::Record)?, settle),
            "the settle band must be ordered after the act log recorder, or it reports a head \
             the sim has not written yet",
        );
    }
    Ok(())
}
