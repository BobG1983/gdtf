use bevy::prelude::*;
use gdtf_battle_sim::{
    acts::{ShoveRequested, StabilizeDownedRequested},
    ganger::LifeState,
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level},
    test_support::GangerEntityBuilder,
};
use gdtf_test_utils::{MessageProbePlugin, probed};

use super::{ContextualActAppExt, PendingContextualIntents, ShoveAct, StabilizeAct};
use crate::SelectedShooter;

fn shove_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_contextual_act::<ShoveAct>();
    app.init_resource::<SelectedShooter>();
    app.world_mut().insert_resource(BattleInProgress);
    app.add_plugins(MessageProbePlugin::<ShoveRequested>::default());
    app
}

fn shoves(app: &App) -> Vec<ShoveRequested> {
    probed::<ShoveRequested>(app)
}

#[test]
fn drain_emits_request_for_selection_same_update() {
    let mut app = shove_app();
    let actor = app.world_mut().spawn_empty().id();
    let target = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));

    app.world_mut()
        .resource_mut::<PendingContextualIntents<ShoveAct>>()
        .push(target);
    app.update();

    let emitted = shoves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "one push with a selection drains to exactly one request the same update",
    );
    assert_eq!(
        emitted[0],
        ShoveRequested::new(actor, target),
        "the request carries the SelectedShooter as actor over the pushed target",
    );
    assert!(
        app.world()
            .resource::<PendingContextualIntents<ShoveAct>>()
            .is_empty(),
        "the queue is emptied by the drain (a press is acted on exactly once)",
    );
}

#[test]
fn drain_emits_nothing_without_selection() {
    let mut app = shove_app();
    let target = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingContextualIntents<ShoveAct>>()
        .push(target);
    app.update();

    assert!(
        shoves(&app).is_empty(),
        "no selection means no emitted request",
    );
    assert!(
        app.world()
            .resource::<PendingContextualIntents<ShoveAct>>()
            .is_empty(),
        "the queue is still consumed (a stale press never lingers)",
    );
}

fn stabilize_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_contextual_act::<StabilizeAct>();
    app.init_resource::<SelectedShooter>();
    app.world_mut().insert_resource(BattleInProgress);
    app.add_plugins(MessageProbePlugin::<StabilizeDownedRequested>::default());
    app
}

#[test]
fn downed_ganger_stays_a_valid_stabilize_target() {
    let mut app = stabilize_app();
    let actor = app.world_mut().spawn_empty().id();
    let downed = GangerEntityBuilder::new()
        .faction(Faction::new(0))
        .life_state(LifeState::Downed)
        .at(CellLevel::new(Cell::new(1, 0), Level::new(0)))
        .spawn(app.world_mut());
    app.world_mut().insert_resource(SelectedShooter::new(actor));

    app.world_mut()
        .resource_mut::<PendingContextualIntents<StabilizeAct>>()
        .push(downed);
    app.update();

    let emitted = probed::<StabilizeDownedRequested>(&app);
    assert_eq!(
        emitted.len(),
        1,
        "a pushed Downed target drains to exactly one stabilize request the same update",
    );
    assert_eq!(
        emitted[0],
        StabilizeDownedRequested::new(actor, downed),
        "the Downed ganger is carried through as the stabilize TARGET — the actor/selection \
         gate never filters the target path",
    );
}

#[test]
fn drain_is_inert_without_a_live_battle() {
    let mut app = shove_app();
    app.world_mut().remove_resource::<BattleInProgress>();
    let actor = app.world_mut().spawn_empty().id();
    let target = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));

    app.world_mut()
        .resource_mut::<PendingContextualIntents<ShoveAct>>()
        .push(target);
    app.update();

    assert!(
        shoves(&app).is_empty(),
        "pre-battle the drain must not emit",
    );
    assert!(
        !app.world()
            .resource::<PendingContextualIntents<ShoveAct>>()
            .is_empty(),
        "pre-battle the queue is untouched (the run_if gate keeps the drain inert)",
    );
}
