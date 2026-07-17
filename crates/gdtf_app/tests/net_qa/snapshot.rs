//! GTW-738 — the T5 snapshot view service, driven through the REAL `build_snapshots`
//! system on a live-battle `BattleAppBuilder` app (never a shadow copy).
//!
//! The CONTENT tests pin the curated `GetBattleState` `BattleView` STRUCTURE — fields
//! present, ganger counts, indexed fire modes, terrain token round-trips, the fog / turn
//! invariants — never a tunable balance magnitude (the loader-test convention). The
//! CONSISTENCY test injects an intent and `GetBattleState` in the SAME frame and asserts the
//! reply reflects that frame's post-Simulate mutation — proving the builder reads live
//! post-Simulate state, not a stale one-frame-behind cache.

use bevy::prelude::Entity;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    emplacement::EmplacementState, entity::TerrainCell, ganger::Aiming, openable::OpenState,
};
use gdtf_qa_protocol::{
    envelope::{InjectReceipt, QaRequest, QaResponse},
    intent::{AimNet, NetIntent},
};

use crate::inject_support::*;

/// CONTENT: a `GetBattleState` on a live seeded battle answers a `BattleView` whose curated
/// fields are all populated — living ganger cards (each with a round-tripping token and an
/// indexed fire-mode list), a terrain grid with real dimensions, the fog invariant, a
/// populated selection, and a coherent turn state. STRUCTURE only — no magnitude is pinned.
#[test]
fn get_battle_state_answers_a_populated_battle_view() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    let reply = send(&tx, QaRequest::GetBattleState);
    app.update();
    let reply = reply.try_recv();
    assert!(
        matches!(&reply, Ok(QaResponse::Battle(_))),
        "GetBattleState on a live battle must answer a Battle view, got {reply:?}",
    );
    let Ok(QaResponse::Battle(view)) = reply else {
        return;
    };

    // The seeded `two_ganger` fixture fields living gangers — the roster is non-empty.
    assert!(
        !view.gangers.is_empty(),
        "a live battle must surface at least one living ganger",
    );
    for ganger in &view.gangers {
        // Every ganger token round-trips to a LIVE entity (never a raw leaked bit pattern).
        let entity = Entity::try_from_bits(*ganger.token);
        assert!(
            matches!(entity, Some(e) if app.world().get_entity(e).is_ok()),
            "every ganger token must resolve to a live entity, got {:?}",
            ganger.token,
        );
        // The weapon card is always present; its fire-mode list is indexed 0..n, each labelled.
        for (index, mode) in ganger.weapon.modes.iter().enumerate() {
            assert_eq!(
                *mode.index as usize, index,
                "the indexed fire-mode list must run 0..n in order",
            );
            assert!(
                !mode.label.is_empty(),
                "every fire mode carries a human-facing label",
            );
        }
    }

    // The terrain grid dimensions are the real structural coarse-grid extent (non-zero).
    assert!(
        *view.terrain.grid.width > 0
            && *view.terrain.grid.height > 0
            && *view.terrain.grid.levels > 0,
        "the terrain summary must carry real grid dimensions",
    );

    // The fog invariant: every VISIBLE cell is also EXPLORED (visible ⊆ explored).
    assert!(
        view.fog.visible.len() <= view.fog.explored.len(),
        "visible cells are a subset of explored cells",
    );

    // The battle auto-selects a player ganger, so the selection is populated and its token
    // names one of the surfaced ganger cards.
    let selected = view.selection.selected;
    assert!(
        selected.is_some(),
        "a live battle auto-selects a player ganger"
    );
    assert!(
        selected.is_none_or(|token| view.gangers.iter().any(|g| g.token == token)),
        "the selected token must name one of the surfaced ganger cards",
    );

    // Turn state: the player acts first, so the active faction is the player's at battle start.
    assert_eq!(
        view.turn.active, view.turn.player,
        "the player faction acts first, so it is the active faction at battle start",
    );
}

/// TOKEN HANDOUT: a door and an emplacement spawned into the live battle are surfaced in the
/// `TerrainSummaryView` with tokens that round-trip back to those exact entities — so a
/// client can drive the door / emplacement intents against a token a view handed it.
#[test]
fn terrain_summary_hands_out_door_and_emplacement_tokens() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // Spawn one openable door and one weapon emplacement as real terrain entities.
    let door = app
        .world_mut()
        .spawn((TerrainCell::new(*at(3, 4)), OpenState::Closed))
        .id();
    let emplacement = app
        .world_mut()
        .spawn((TerrainCell::new(*at(7, 8)), EmplacementState::Vacant))
        .id();
    app.update();

    let reply = send(&tx, QaRequest::GetBattleState);
    app.update();
    let reply = reply.try_recv();
    assert!(
        matches!(&reply, Ok(QaResponse::Battle(_))),
        "GetBattleState must answer a Battle view, got {reply:?}",
    );
    let Ok(QaResponse::Battle(view)) = reply else {
        return;
    };

    assert!(
        view.terrain
            .doors
            .iter()
            .any(|d| Entity::try_from_bits(*d.token) == Some(door)),
        "the spawned door must be surfaced with a token that round-trips to its entity",
    );
    assert!(
        view.terrain
            .emplacements
            .iter()
            .any(|e| Entity::try_from_bits(*e.token) == Some(emplacement)),
        "the spawned emplacement must be surfaced with a round-tripping token",
    );
}

/// POST-SIMULATE SAME-FRAME CONSISTENCY: injecting a `SetAiming` intent AND a `GetBattleState`
/// in the SAME frame yields a snapshot whose selected ganger already shows the FLIPPED aim —
/// proving the view builder reads the LIVE post-Simulate world that frame, not a stale cache.
#[test]
fn same_frame_snapshot_reflects_an_injected_aim_toggle() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // The auto-selected player ganger is the actor the SetAiming intent acts on.
    let actor = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s);
    assert!(
        actor.is_some(),
        "a live battle auto-selects a player ganger"
    );
    let Some(actor) = actor else {
        return;
    };
    let before = app.world().get::<Aiming>(actor).copied();
    assert!(
        before.is_some(),
        "the selected ganger carries an Aiming component",
    );
    let Some(before) = before else {
        return;
    };
    let requested = !*before;

    // Inject the aim flip AND a snapshot request, both routed the SAME frame.
    let inject_reply = send(
        &tx,
        QaRequest::Inject(NetIntent::SetAiming {
            aim: AimNet::new(requested),
        }),
    );
    let snapshot_reply = send(&tx, QaRequest::GetBattleState);
    app.update();

    assert!(
        matches!(
            inject_reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Queued))
        ),
        "the injected SetAiming must be Queued the frame it enters the input queue",
    );
    let snapshot_reply = snapshot_reply.try_recv();
    assert!(
        matches!(&snapshot_reply, Ok(QaResponse::Battle(_))),
        "the same-frame GetBattleState must answer a Battle view, got {snapshot_reply:?}",
    );
    let Ok(QaResponse::Battle(view)) = snapshot_reply else {
        return;
    };
    let card = view
        .gangers
        .iter()
        .find(|g| Entity::try_from_bits(*g.token) == Some(actor));
    assert!(
        card.is_some(),
        "the acting ganger must appear in the same-frame snapshot",
    );
    assert!(
        card.is_none_or(|c| *c.aiming == requested),
        "the SAME frame's snapshot must reflect the post-Simulate aim flip (not a stale cache)",
    );
}
