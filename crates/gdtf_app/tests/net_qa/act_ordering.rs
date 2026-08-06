//! The act commands must claim before the act bus drains and settle after the sim records.

use bevy::{
    app::Update,
    ecs::schedule::{
        IntoSystemSet, NodeId, Schedule, ScheduleGraph, Schedules, SystemKey, SystemSet,
        graph::Direction,
    },
};
use gdtf_app::test_support::ActCommandSystems;
use gdtf_battle_input::auto_select_first_player_ganger;
use gdtf_battle_sim::occupancy_sync::SimSystems;
use gdtf_qa_command::dispatch::QaCommandSystems;

use super::socket_support::{TestError, TestResult, battle_app_listening};

/// One act command claims and settles, so each band holds eleven systems.
const CLASSIC_ACTS: usize = 11;

/// The node a set is, or a failure saying the app never registered it.
fn set_node(graph: &ScheduleGraph, set: impl SystemSet) -> Result<NodeId, TestError> {
    graph
        .system_sets
        .get_key(set.intern())
        .map(NodeId::Set)
        .ok_or_else(|| "the app must register this set, or its ordering means nothing".into())
}

/// The systems a set holds directly or through a nested set.
fn members(graph: &ScheduleGraph, set: impl SystemSet) -> Result<Vec<SystemKey>, TestError> {
    graph
        .systems_in_set(set.intern())
        .map(|keys| keys.iter().copied().collect())
        .map_err(|fault| format!("the app must register this set: {fault:?}").into())
}

/// The system whose name ends in `named`, when the schedule holds exactly one.
///
/// Names come from the built schedule, not the graph: building moves each system out of
/// `ScheduleGraph::systems`, which is why the graph's own iterator comes back empty.
fn a_system_named(schedule: &Schedule, named: &str) -> Result<SystemKey, TestError> {
    let Ok(systems) = schedule.systems() else {
        return Err("the schedule has not been initialised".into());
    };
    let mut found: Vec<SystemKey> = systems
        .filter(|(_, system)| system.name().to_string().ends_with(named))
        .map(|(key, _)| key)
        .collect();
    match found.pop() {
        Some(key) if found.is_empty() => Ok(key),
        Some(_) => Err(format!("`{named}` is registered more than once").into()),
        None => Err(format!("`{named}` is not registered in this schedule").into()),
    }
}

/// Whether `node` is `wanted`, or a set holding it however deeply nested.
fn covers(graph: &ScheduleGraph, node: NodeId, wanted: SystemKey) -> bool {
    let target = NodeId::System(wanted);
    let mut seen: Vec<NodeId> = Vec::new();
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if node == target {
            return true;
        }
        if seen.contains(&node) {
            continue;
        }
        seen.push(node);
        pending.extend(
            graph
                .hierarchy()
                .graph()
                .neighbors_directed(node, Direction::Outgoing),
        );
    }
    false
}

/// Whether the schedule carries a dependency edge out of `from` that covers `to`.
fn ordered_before(graph: &ScheduleGraph, from: NodeId, to: SystemKey) -> bool {
    graph
        .dependency()
        .graph()
        .neighbors_directed(from, Direction::Outgoing)
        .any(|next| covers(graph, next, to))
}

#[test]
fn every_act_claim_runs_before_the_act_bus_drains() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let Some(schedules) = app.world().get_resource::<Schedules>() else {
        unreachable!("a built app carries its schedules");
    };
    let Some(update) = schedules.get(Update) else {
        unreachable!("the app carries an Update schedule");
    };
    let graph = update.graph();

    let claims = members(graph, ActCommandSystems::Claim)?;
    assert_eq!(
        claims.len(),
        CLASSIC_ACTS,
        "every classic act must register its claim system in the band, or the band's ordering \
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
fn every_command_claims_after_the_game_has_settled_its_selection() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let Some(schedules) = app.world().get_resource::<Schedules>() else {
        unreachable!("a built app carries its schedules");
    };
    let Some(update) = schedules.get(Update) else {
        unreachable!("the app carries an Update schedule");
    };
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
    let Some(schedules) = app.world().get_resource::<Schedules>() else {
        unreachable!("a built app carries its schedules");
    };
    let Some(update) = schedules.get(Update) else {
        unreachable!("the app carries an Update schedule");
    };
    let graph = update.graph();

    let settles = members(graph, ActCommandSystems::Settle)?;
    assert_eq!(
        settles.len(),
        CLASSIC_ACTS,
        "every classic act must register its settle system in the band",
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
