//! Reading the built `Update` schedule: which set holds a system, and what runs before what.

use bevy::{
    app::{App, Update},
    ecs::schedule::{
        NodeId, Schedule, ScheduleGraph, Schedules, SystemKey, SystemSet, graph::Direction,
    },
};

use super::socket_support::TestError;

/// The built `Update` schedule of an app that has already run a frame.
pub(crate) fn update_schedule(app: &App) -> Result<&Schedule, TestError> {
    let Some(schedules) = app.world().get_resource::<Schedules>() else {
        return Err("a built app carries its schedules".into());
    };
    schedules
        .get(Update)
        .ok_or_else(|| "the app carries an Update schedule".into())
}

/// The node a set is, or a failure saying the app never registered it.
pub(crate) fn set_node(graph: &ScheduleGraph, set: impl SystemSet) -> Result<NodeId, TestError> {
    graph
        .system_sets
        .get_key(set.intern())
        .map(NodeId::Set)
        .ok_or_else(|| "the app must register this set, or its ordering means nothing".into())
}

/// The systems a set holds directly or through a nested set.
pub(crate) fn members(
    graph: &ScheduleGraph,
    set: impl SystemSet,
) -> Result<Vec<SystemKey>, TestError> {
    graph
        .systems_in_set(set.intern())
        .map(|keys| keys.iter().copied().collect())
        .map_err(|fault| format!("the app must register this set: {fault:?}").into())
}

/// The system whose name ends in `named`, when the schedule holds exactly one.
///
/// Names come from the built schedule, not the graph: building moves each system out of
/// `ScheduleGraph::systems`, which is why the graph's own iterator comes back empty.
pub(crate) fn a_system_named(schedule: &Schedule, named: &str) -> Result<SystemKey, TestError> {
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
pub(crate) fn covers(graph: &ScheduleGraph, node: NodeId, wanted: SystemKey) -> bool {
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
pub(crate) fn ordered_before(graph: &ScheduleGraph, from: NodeId, to: SystemKey) -> bool {
    graph
        .dependency()
        .graph()
        .neighbors_directed(from, Direction::Outgoing)
        .any(|next| covers(graph, next, to))
}
