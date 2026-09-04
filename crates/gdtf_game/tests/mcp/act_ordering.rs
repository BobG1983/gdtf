//! The act commands must claim after the turn tally, before the act bus drains, and settle after
//! the sim records.

use bevy::ecs::schedule::{IntoSystemSet, NodeId};
use cobalt_mcp_command::dispatch::QaCommandSystems;
use gdtf_battle_input::{
    InputSystems, auto_select_first_player_ganger, clear_downed_selection,
    contextual::ContextualActSystems,
};
use gdtf_battle_sim::occupancy_sync::SimSystems;
use gdtf_game::test_support::{ActCommandSystems, ContextualPanelSystems, count_turn_changes};

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

/// The three view controls push a view intent from outside the act claim band, so each one
/// carries its own edge to the drain.
const VIEW_CONTROLS: [&str; 3] = [
    "handle_view_level_up",
    "handle_view_level_down",
    "handle_view_toggle_full_view",
];

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
        "every act command must register its claim system in the band, or the band's ordering \
         does not apply to it",
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
fn every_view_control_runs_before_the_act_bus_drains() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let drain = a_system_named(update, "dispatch_act_intents")?;
    let banded = members(graph, ActCommandSystems::Claim)?;
    let gathering = members(graph, InputSystems::Gather)?;
    for named in VIEW_CONTROLS {
        let handler = a_system_named(update, named)?;
        assert!(
            gathering.contains(&handler),
            "`{named}` gathers a view intent, so it belongs in InputSystems::Gather with the rest \
             of the input the frame collects; outside that set it runs against whatever else the \
             executor picks and only its own drain edge holds it in place",
        );
        assert!(
            !banded.contains(&handler),
            "`{named}` answers its own call rather than an act-log window, so joining the act \
             claim band would put it in the settle band's count as well",
        );
        assert!(
            ordered_before(graph, NodeId::System(handler), drain),
            "`{named}` pushes a view intent from outside the claim band, so its own edge to \
             dispatch_act_intents is the only thing draining that intent in the frame it was \
             pushed; without it the two share InputSystems::Gather and the view moves a frame late",
        );
    }
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
fn the_panel_scans_its_offers_before_the_game_clears_a_downed_selection() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let clear = a_system_named(update, "clear_downed_selection")?;
    assert!(
        covers(
            graph,
            set_node(graph, clear_downed_selection.into_system_set())?,
            clear
        ),
        "the app must register clear_downed_selection, or ordering against it orders against \
         nothing",
    );
    let scans = members(graph, ContextualPanelSystems::Offer)?;
    assert!(
        !scans.is_empty(),
        "the panel must register an offer scan per contextual act family, or ordering against \
         ContextualPanelSystems::Offer orders against nothing",
    );
    assert!(
        ordered_before(
            graph,
            set_node(graph, ContextualPanelSystems::Offer)?,
            clear
        ),
        "the offer scan reads the selection and the clear writes it, so the scan must be ordered \
         first; unordered, a downed selection can be cleared and re-picked before the scan runs \
         and every act's alive-actor term is asked about a different ganger",
    );
    Ok(())
}

#[test]
fn every_act_claim_runs_after_the_turn_tally_has_counted_this_frame() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let tally = set_node(graph, count_turn_changes.into_system_set())?;
    let counter = a_system_named(update, "count_turn_changes")?;
    assert!(
        covers(graph, tally, counter),
        "the app must register count_turn_changes, or ordering against it orders against nothing",
    );
    let claims = members(graph, ActCommandSystems::Claim)?;
    assert!(
        !claims.is_empty(),
        "the act commands must register their claim systems, or this ordering means nothing",
    );
    for claim in claims {
        assert!(
            ordered_before(graph, tally, claim),
            "the claim band must be ordered after count_turn_changes; that edge is also the only \
             thing holding the tally before dispatch_act_intents, so without it a wait parked in \
             the same frame as an act command can read the count from after the hand-over the act \
             caused and then hold out for a second one",
        );
    }
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
