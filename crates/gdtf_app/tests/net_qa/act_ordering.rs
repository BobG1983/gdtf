//! The act commands must claim before the act bus drains and settle after the sim records.

use bevy::ecs::schedule::IntoSystemSet;
use gdtf_app::test_support::{ActCommandSystems, ContextualPanelSystems};
use gdtf_battle_input::{auto_select_first_player_ganger, contextual::ContextualActSystems};
use gdtf_battle_sim::occupancy_sync::SimSystems;
use gdtf_qa_command::dispatch::QaCommandSystems;

use super::{
    schedule_support::{
        a_system_named, covers, members, ordered_before, set_node, update_schedule,
    },
    socket_support::{TestResult, battle_app_listening},
};

/// One command claims and settles, so each band holds the nineteen acts plus
/// `input.click_cell`.
const BANDED_ACTS: usize = 20;

/// The eight acts that fire whatever the contextual panel is offering.
const CONTEXTUAL_ACTS: usize = 8;

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
fn every_act_claim_runs_before_the_contextual_act_bus_drains() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let drains = members(graph, ContextualActSystems::Drain)?;
    assert!(
        !drains.is_empty(),
        "the input plugin must register a drain per contextual act family, or ordering against \
         the band orders against nothing",
    );
    let claims = set_node(graph, ActCommandSystems::Claim)?;
    for drain in drains {
        assert!(
            ordered_before(graph, claims, drain),
            "the claim band must be ordered before the contextual drain; without that edge the \
             two only share InputSystems::Gather and a contextual act command's push lands a \
             frame late",
        );
    }
    Ok(())
}

#[test]
fn every_contextual_act_claim_runs_after_the_panel_has_scanned_this_frame_s_offers() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let scans = members(graph, ContextualPanelSystems::Offer)?;
    assert!(
        !scans.is_empty(),
        "the panel must register an offer scan per contextual act family, or ordering against \
         the band orders against nothing",
    );
    let claims = members(graph, ActCommandSystems::ContextualClaim)?;
    assert_eq!(
        claims.len(),
        CONTEXTUAL_ACTS,
        "every command that fires what the panel offers must register its claim system in the \
         contextual band, or the band's ordering does not apply to it",
    );
    let offer = set_node(graph, ContextualPanelSystems::Offer)?;
    for claim in claims {
        assert!(
            ordered_before(graph, offer, claim),
            "a contextual act command reads the same per-frame offer the scan writes, so its \
             claim must be ordered after it; unordered, a Res read and a ResMut write in one \
             schedule run in whichever order the executor picks",
        );
    }
    Ok(())
}

#[test]
fn the_offers_read_runs_after_the_panel_has_settled_its_buttons() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let read = a_system_named(update, "handle_battle_offers")?;
    assert!(
        ordered_before(
            graph,
            set_node(graph, ContextualPanelSystems::Toggle)?,
            read
        ),
        "battle.offers tells a client an offer it reads is a button on screen; the panel chains \
         its toggle after its offer scan, so ordering the read after the toggle is what puts it \
         after the scan's write, and unordered the reply can disagree with the buttons",
    );
    Ok(())
}

#[test]
fn the_panel_scans_its_offers_before_the_picker_recomputes_the_hover() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let picker = a_system_named(update, "pick_hovered_cell")?;
    assert!(
        ordered_before(
            graph,
            set_node(graph, ContextualPanelSystems::Offer)?,
            picker
        ),
        "act.throw_grenade's summary and the QA guide both tell a client the offer it reads was \
         scanned against the cell the previous frame resolved; run the scan after the pick and \
         both go wrong by a frame while the suite stays green",
    );
    Ok(())
}

#[test]
fn the_classic_act_claims_are_left_free_of_the_panel_s_offer_scan() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let contextual = members(graph, ActCommandSystems::ContextualClaim)?;
    let offer = set_node(graph, ContextualPanelSystems::Offer)?;
    let classic = members(graph, ActCommandSystems::Claim)?
        .into_iter()
        .filter(|claim| !contextual.contains(claim));
    for claim in classic {
        assert!(
            !ordered_before(graph, offer, claim),
            "a classic act command never reads an offer, so widening the offer edge to the whole \
             claim band would reorder it for nothing",
        );
    }
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
