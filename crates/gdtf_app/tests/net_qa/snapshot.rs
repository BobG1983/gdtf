//! GTW-738 — the T5 snapshot view service, driven through the REAL `build_snapshots`
//! system on a live-battle `BattleAppBuilder` app (never a shadow copy).
//!
//! The CONTENT tests pin the curated `GetBattleState` `BattleView` STRUCTURE — fields
//! present, ganger counts, indexed fire modes, terrain token round-trips, the fog / turn
//! invariants — never a tunable balance magnitude (the loader-test convention). The
//! CONSISTENCY test injects an intent and `GetBattleState` in the SAME frame and asserts the
//! reply reflects that frame's post-Simulate mutation — proving the builder reads live
//! post-Simulate state, not a stale one-frame-behind cache.

use bevy::prelude::{Entity, Text};
use gdtf_battle_input::{PanelNavOrder, SelectedShooter};
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

    // The fog invariant: the visible set is a subset of the explored set, so the visible
    // count never exceeds the explored count.
    assert!(
        *view.fog.visible_count <= *view.fog.explored_count,
        "visible cells are a subset of explored cells",
    );

    // SIZE (GTW-763): the whole curated BattleView stays compact on a real live battle. The
    // fog was the only unbounded field and now carries counts, so the snapshot is
    // low-thousands of wire bytes, not the ~93k full-fog dump that made query_state unusable.
    // Pinning the WHOLE view size (not just fog) also trips if a future field re-introduces an
    // unbounded per-cell list.
    let Ok(encoded) = gdtf_qa_protocol::framing::encode(&view) else {
        unreachable!("a live battle view encodes to the wire: {view:?}");
    };
    assert!(
        encoded.len() < 8_000,
        "the curated battle view must stay compact (GTW-763); got {} wire bytes",
        encoded.len(),
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

/// PANEL BUTTON HANDOUT (GTW-789): focus-navigable HUD buttons spawned into the live battle
/// are surfaced in the `buttons` list — each with a focus token that round-trips to its
/// entity, its `PanelNavOrder` ordinal, and its caption label READ off the button's `Text`
/// child (the shape `spawn_button` produces) — sorted by Tab-chain ordinal. So a client can
/// `SetFocus` onto a real panel button a view handed it (unblocking GTW-782's focus-ring
/// capture over the `net_qa` wire).
#[test]
fn panel_buttons_are_handed_out_with_tokens_ordinals_and_labels() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // Spawn two focus-navigable buttons shaped the way `spawn_button` shapes them: a
    // `PanelNavOrder` root with its caption on a `Text` CHILD. Spawn them OUT of Tab order
    // (Reload before End Turn) to prove the handout re-sorts by ordinal.
    let reload_text = app.world_mut().spawn(Text::new("Reload")).id();
    let reload = app.world_mut().spawn(PanelNavOrder::new(100)).id();
    app.world_mut().entity_mut(reload).add_child(reload_text);
    let end_turn_text = app.world_mut().spawn(Text::new("End Turn")).id();
    let end_turn = app.world_mut().spawn(PanelNavOrder::new(2)).id();
    app.world_mut()
        .entity_mut(end_turn)
        .add_child(end_turn_text);
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

    // Both spawned buttons are surfaced, each token round-tripping to its exact entity.
    let end_turn_view = view
        .buttons
        .iter()
        .find(|b| Entity::try_from_bits(*b.token) == Some(end_turn));
    let reload_view = view
        .buttons
        .iter()
        .find(|b| Entity::try_from_bits(*b.token) == Some(reload));
    assert!(
        end_turn_view.is_some() && reload_view.is_some(),
        "every focus button must be surfaced with a token that round-trips to its entity",
    );

    // The ordinal + label are the REAL ones read off the entities (not fabricated).
    assert!(
        end_turn_view.is_none_or(|b| *b.order == 2 && b.label.as_str() == "End Turn"),
        "the End Turn button carries its PanelNavOrder ordinal + caption label",
    );
    assert!(
        reload_view.is_none_or(|b| *b.order == 100 && b.label.as_str() == "Reload"),
        "the Reload button carries its PanelNavOrder ordinal + caption label",
    );

    // The handout is sorted by Tab-chain ordinal (End Turn = 2 before Reload = 100) even
    // though they were spawned Reload-first — the deterministic wire order the topology walks.
    let ordinals: Vec<u16> = view.buttons.iter().map(|b| *b.order).collect();
    let mut sorted = ordinals.clone();
    sorted.sort_unstable();
    assert_eq!(
        ordinals, sorted,
        "the button handout must be sorted by Tab-chain ordinal, got {ordinals:?}",
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
