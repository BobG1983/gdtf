//! The fog shadow is promoted before the combat-log forwarders decide what to print.

use bevy::ecs::schedule::{NodeId, Schedule, SystemKey};
use gdtf_battle_presenter::CombatLogSystems;

use super::{
    schedule_support::{a_system_named, members, ordered_before, update_schedule},
    socket_support::{TestResult, battle_app_listening},
};

/// The name a system was registered under, so a failure says which forwarder is unordered.
fn name_of(schedule: &Schedule, wanted: SystemKey) -> String {
    let Ok(mut systems) = schedule.systems() else {
        return "<the schedule has not been initialised>".to_owned();
    };
    systems.find(|(key, _)| *key == wanted).map_or_else(
        || "<not in this schedule>".to_owned(),
        |(_, system)| system.name().to_string(),
    )
}

#[test]
fn the_shown_fog_is_promoted_before_the_combat_log_forwards_a_line() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let promote = a_system_named(update, "promote_shown_fog")?;
    let forwarders = members(graph, CombatLogSystems::Forward)?;
    assert!(
        !forwarders.is_empty(),
        "the combat-log forwarders must sit in CombatLogSystems::Forward, or ordering the \
         fog promote before that set pins nothing",
    );

    let unordered: Vec<String> = forwarders
        .iter()
        .filter(|forwarder| !ordered_before(graph, NodeId::System(promote), **forwarder))
        .map(|forwarder| name_of(update, *forwarder))
        .collect();
    assert!(
        unordered.is_empty(),
        "promote_shown_fog must run before every combat-log forwarder, or the panel gates a \
         line on a fog shadow promoted for an earlier frame; unordered: {unordered:?}",
    );
    Ok(())
}
