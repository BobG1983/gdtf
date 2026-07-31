//! Registering a command declares `Route` before `Claim` itself.
//!
//! The ordering is not a host's to choose: routing fills the `CommandInbox` and claiming
//! empties it, so a claim scheduled first leaves every call a frame late. Nothing in a
//! running app OBSERVES the edge — a fake host's routing happens outside the schedule, and
//! a real host that forgot `configure_sets` would get whichever order Bevy picked — so the
//! edge is read straight out of the built schedule graph instead.

use bevy::{
    ecs::schedule::{NodeId, SystemSet},
    prelude::*,
};

use crate::{
    dispatch::{QaCommandSystems, register_command},
    test_support::FakePhase,
};

/// Whether the `Update` schedule carries a `before` edge from `Route` to `Claim`.
fn route_runs_before_claim(app: &App) -> bool {
    let Some(schedule) = app.get_schedule(Update) else {
        return false;
    };
    let sets = &schedule.graph().system_sets;
    let (Some(route), Some(claim)) = (
        sets.get_key(QaCommandSystems::Route.intern()),
        sets.get_key(QaCommandSystems::Claim.intern()),
    ) else {
        return false;
    };
    schedule
        .graph()
        .dependency()
        .graph()
        .contains_edge(NodeId::Set(route), NodeId::Set(claim))
}

/// One `register_command` call is enough to order the two sets.
#[test]
fn registering_one_command_orders_route_before_claim() {
    let mut app = App::new();
    register_command::<FakePhase>(&mut app);

    assert!(
        route_runs_before_claim(&app),
        "register_command must chain Route before Claim, so no host can forget to"
    );
}

/// A bare `App` that registered nothing has no such edge — so the test above is reading the
/// registration's work and not something Bevy provides for free.
#[test]
fn a_bare_app_declares_no_such_ordering() {
    let app = App::new();

    assert!(
        !route_runs_before_claim(&app),
        "the ordering must come from register_command, not from App::new"
    );
}
