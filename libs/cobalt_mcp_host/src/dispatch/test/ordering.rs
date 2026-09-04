use bevy::{
    ecs::schedule::{NodeId, SystemSet},
    prelude::*,
};

use crate::{
    dispatch::{McpCommandSystems, register_command},
    test_support::FakePhase,
};

fn route_runs_before_claim(app: &App) -> bool {
    let Some(schedule) = app.get_schedule(Update) else {
        return false;
    };
    let sets = &schedule.graph().system_sets;
    let (Some(route), Some(claim)) = (
        sets.get_key(McpCommandSystems::Route.intern()),
        sets.get_key(McpCommandSystems::Claim.intern()),
    ) else {
        return false;
    };
    schedule
        .graph()
        .dependency()
        .graph()
        .contains_edge(NodeId::Set(route), NodeId::Set(claim))
}

#[test]
fn registering_one_command_orders_route_before_claim() {
    let mut app = App::new();
    register_command::<FakePhase>(&mut app);

    assert!(
        route_runs_before_claim(&app),
        "register_command must chain Route before Claim, so no host can forget to"
    );
}

#[test]
fn a_bare_app_declares_no_such_ordering() {
    let app = App::new();

    assert!(
        !route_runs_before_claim(&app),
        "the ordering must come from register_command, not from App::new"
    );
}
