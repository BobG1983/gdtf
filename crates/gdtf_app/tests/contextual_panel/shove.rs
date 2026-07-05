//! GTW-525 SHOVE button: detection, press, same-update drain.

use bevy::prelude::*;
use gdtf_app::test_support::ShoveButton;
use gdtf_battle_input::contextual::{ContextualActSystems, PendingContextualIntents, ShoveAct};
use gdtf_battle_sim::{Position, acts::ShoveRequested};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

/// Adds the [`ShoveRequested`] probe — the generic GTW-576 `MessageProbe<M>` with its drain
/// at the ORIGINAL observation point (Update, AFTER the GTW-571 contextual drain set),
/// so it pins the DRAIN's same-update emission, not any later sim re-emission.
fn add_shove_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ShoveRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ShoveRequested>.after(ContextualActSystems::Drain),
    );
}

/// The collected [`ShoveRequested`] messages.
fn shoves(app: &App) -> Vec<ShoveRequested> {
    probed::<ShoveRequested>(app)
}

// ---------------------------------------------------------------------------------
// GTW-525 — the SHOVE button: detection (an 8-adjacent alive opposing ganger reveals it,
// WEAKER than Melee — no LOS / no weapon) and press → ShoveRequested through the REAL seam.
// ---------------------------------------------------------------------------------

/// SHOVE detection: a selected actor (any ganger — the plain `spawn_actor`, which carries NO
/// stance / facing, so NO melee is ever offered) with an 8-adjacent, ALIVE, OPPOSING ganger
/// offers Shove — the dedicated Shove button + the panel root become Visible (GTW-525). The gate
/// is WEAKER than Melee's: NO LOS and NO weapon are needed (any ganger can shove any alive
/// opposing neighbour), which is why the LOS-less `spawn_actor` still reveals it.
#[test]
fn adjacent_alive_opposing_offers_shove() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // An ALIVE ENEMY (gang 1) one cell diagonally — 8-adjacent.
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();

    assert!(
        shove_visible(&mut app),
        "an 8-adjacent alive opposing ganger must reveal the Shove button (GTW-525)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Shove must reveal the panel root",
    );
    // Melee needs the actor's stance + facing (the LOS eye); a plain actor carries neither, so
    // Melee is NOT offered even though an alive enemy is adjacent — Shove is the WEAKER gate.
    assert!(
        !melee_visible(&mut app),
        "a stance/facing-less actor offers NO melee, yet Shove still reveals (weaker gate)",
    );
    // The downed-only acts stay hidden — the enemy is ALIVE, not downed.
    assert!(
        !execute_visible(&mut app),
        "an ALIVE enemy is no Execute target (Execute needs a DOWNED enemy) -> hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
}

/// SHOVE detection — the gate is adjacency + alive + opposing (NO downed / NO ally): a
/// non-adjacent alive enemy, an adjacent ALLY, and an adjacent DOWNED enemy each offer NO shove
/// (the button stays hidden). Discriminating: moving the alive enemy INTO 8-adjacency reveals it.
#[test]
fn non_adjacent_ally_or_downed_does_not_offer_shove() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // An alive ENEMY far away (Chebyshev > 1) — not a shove candidate.
    let enemy = spawn_alive_enemy(&mut app, 20, 20, 1);
    // An alive ALLY 8-adjacent — same faction, never a shove target.
    spawn_alive_enemy(&mut app, 5, 6, 0);
    // A DOWNED enemy 8-adjacent — not ALIVE, so no shove (the deliberate gate needs `is_active`).
    spawn_downed(&mut app, 4, 4, 1, None);
    app.update();

    assert!(
        !shove_visible(&mut app),
        "a non-adjacent enemy + an adjacent ally + an adjacent DOWNED enemy offer NO shove",
    );

    // Move the alive enemy INTO 8-adjacency — the SAME enemy now reveals the Shove button (the
    // gate is adjacency + alive + opposing, not mere presence).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(enemy) {
        *pos = at(6, 6);
    }
    app.update();
    assert!(
        shove_visible(&mut app),
        "moving the alive opposing enemy into 8-adjacency reveals the Shove button (discriminating)",
    );
}

/// PRESS → INTENT: with a Shove target offered (an 8-adjacent alive opposing ganger), pressing the
/// Shove button drives the REAL GTW-571 stack (button -> the generic press router ->
/// `PendingContextualIntents<ShoveAct>` -> the act's generic drain) to emit exactly one
/// `ShoveRequested` for the `SelectedShooter` as shover over the carried target (GTW-525) —
/// driven THROUGH the button/press path, NOT a synthetic `ShoveRequested` emit. The emitted
/// request is the DELIBERATE form (`ShoveSource::Deliberate`).
///
/// Pin-discriminating: dropping the act's press registration (or the offer scan that fills the
/// target) leaves the queue empty and emits zero messages, failing the asserts.
#[test]
fn pressing_shove_emits_shove_requested_for_target() {
    let mut app = battle_running_app();
    add_shove_probe(&mut app);
    let shover = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_alive_enemy(&mut app, 6, 6, 1);

    // First update: detection reveals the Shove button + fills the target offer.
    app.update();
    assert!(
        shove_visible(&mut app),
        "sanity: the Shove button is offered before the press",
    );
    let Some(shove_btn) = single_with::<ShoveButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `shove_visible` assert above.
        return;
    };

    // Drive the press, then update: the generic press router pushes the offered target and the
    // act's generic drain emits ShoveRequested the SAME update.
    press_ui_button(&mut app, shove_btn);
    app.update();

    let emitted = shoves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Shove with a target offered must emit exactly one ShoveRequested",
    );
    assert_eq!(
        emitted[0].shover, shover,
        "the shover is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried opposing neighbour",
    );
}

/// GTW-571 AC-4 — the SAME-FRAME pin after the drain split: a contextual press queued this
/// update is drained THIS update (press -> the act's generic press router -> the buffered
/// `PendingContextualIntents<ShoveAct>` queue -> the act's generic drain -> `ShoveRequested`),
/// because the panel's Press set is EXPLICITLY ordered `.before` the input crate's
/// `ContextualActSystems::Drain` set (which itself precedes `dispatch_act_intents` and the sim
/// band — no ambiguous orderings anywhere on the path, `bevy-traps.md` #3).
///
/// One press + ONE `app.update()` observes EXACTLY one `ShoveRequested` AND an EMPTIED per-act
/// queue. Pin-discriminating: a drain lagging one frame (a missing/reversed set edge) would
/// leave the queue non-empty and the probe at zero after the single update.
#[test]
fn contextual_press_drains_the_same_update_it_was_queued() {
    let mut app = battle_running_app();
    add_shove_probe(&mut app);
    let shover = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_alive_enemy(&mut app, 6, 6, 1);

    // First update: the offer scan reveals the Shove button + fills the offer.
    app.update();
    assert!(
        shove_visible(&mut app),
        "sanity: the Shove button is offered before the press",
    );
    let Some(shove_btn) = single_with::<ShoveButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `shove_visible` assert above.
        return;
    };

    // ONE press, ONE update — the same-frame contract under test.
    press_ui_button(&mut app, shove_btn);
    app.update();

    let emitted = shoves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the press queued this update must be drained to its *Requested THIS update",
    );
    assert_eq!(
        emitted[0].shover, shover,
        "the shover is the SelectedShooter"
    );
    assert_eq!(
        emitted[0].target, target,
        "the target is the offered opposing neighbour",
    );
    assert!(
        app.world()
            .resource::<PendingContextualIntents<ShoveAct>>()
            .is_empty(),
        "the per-act queue is EMPTIED by the same-update drain (acted on exactly once)",
    );
}
