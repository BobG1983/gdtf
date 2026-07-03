//! Tests for the GENERIC contextual-act seam (GTW-571): the registrar wires the queue +
//! message buffer + drain, the drain emits `A::request(actor, target)` for the
//! selection the SAME update a push lands, and it fails closed without a selection.
//!
//! Driven over the REAL registration path (`add_contextual_act::<ShoveAct>()`) — never
//! a shadow copy of the drain. Every `app.world_mut()` mutation is in a test body (the
//! `bevy-traps.md` #7 carve-out).

use bevy::prelude::*;
use gdtf_battle_sim::{BattleInProgress, acts::ShoveRequested};

use super::{ContextualActAppExt, PendingContextualIntents, ShoveAct, drain_contextual_intents};
use crate::SelectedShooter;

/// Collected [`ShoveRequested`] messages (probe) — read AFTER the act's drain so the
/// assert observes the SAME update's emission.
#[derive(Resource, Default)]
struct ShoveProbe(Vec<ShoveRequested>);

/// A minimal app with the REAL `add_contextual_act::<ShoveAct>()` registration, the
/// live-battle witness, and the shove probe ordered after the act's drain.
fn shove_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_contextual_act::<ShoveAct>();
    app.init_resource::<SelectedShooter>();
    app.init_resource::<ShoveProbe>();
    app.world_mut().insert_resource(BattleInProgress);
    app.add_systems(
        Update,
        (|mut reader: MessageReader<ShoveRequested>, mut probe: ResMut<ShoveProbe>| {
            probe.0.extend(reader.read().copied());
        })
        .after(drain_contextual_intents::<ShoveAct>),
    );
    app
}

/// The collected [`ShoveRequested`] messages.
fn shoves(app: &App) -> Vec<ShoveRequested> {
    app.world()
        .get_resource::<ShoveProbe>()
        .map(|probe| probe.0.clone())
        .unwrap_or_default()
}

/// With a selection, a pushed target is drained the SAME update into exactly one
/// `A::request(actor, target)` — and the queue is left empty (acted on exactly once).
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

/// Without a selection the drain consumes the queue but emits NOTHING (fail-closed —
/// the shape every contextual arm has always had).
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

/// Pre-battle (no `BattleInProgress`) the drain is inert: the queue keeps its push and
/// nothing is emitted (the registrar's `run_if` gate).
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
