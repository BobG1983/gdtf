//! Execute/stabilize on adjacent downed: detection + press→intent.

use bevy::prelude::*;
use gdtf_app::test_support::{ContextualPanelRoot, ExecuteButton, OpenDoorButton, StabilizeButton};
use gdtf_battle_input::contextual::ContextualActSystems;
use gdtf_battle_sim::{
    Position,
    acts::{ExecuteDownedRequested, StabilizeDownedRequested},
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

/// Adds the [`ExecuteDownedRequested`] probe — the generic GTW-576 `MessageProbe<M>` with its drain
/// at the ORIGINAL observation point (Update, AFTER the GTW-571 contextual drain set),
/// so it pins the DRAIN's same-update emission, not any later sim re-emission.
fn add_execute_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ExecuteDownedRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ExecuteDownedRequested>.after(ContextualActSystems::Drain),
    );
}

/// The collected [`ExecuteDownedRequested`] messages.
fn executes(app: &App) -> Vec<ExecuteDownedRequested> {
    probed::<ExecuteDownedRequested>(app)
}

/// Adds the [`StabilizeDownedRequested`] probe — the generic GTW-576 `MessageProbe<M>` with its drain
/// at the ORIGINAL observation point (Update, AFTER the GTW-571 contextual drain set),
/// so it pins the DRAIN's same-update emission, not any later sim re-emission.
fn add_stabilize_probe(app: &mut App) {
    app.init_resource::<MessageProbe<StabilizeDownedRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<StabilizeDownedRequested>.after(ContextualActSystems::Drain),
    );
}

/// The collected [`StabilizeDownedRequested`] messages.
fn stabilizes(app: &App) -> Vec<StabilizeDownedRequested> {
    probed::<StabilizeDownedRequested>(app)
}

// ---------------------------------------------------------------------------------
// Detection AC — a selected actor's actionable downed neighbours drive the panel's
// reactive show/hide.
// ---------------------------------------------------------------------------------

/// EXECUTE condition: a selected actor with an 8-adjacent downed ENEMY (different faction) offers
/// Execute — the Execute button + the panel root become Visible while the Stabilize button stays
/// Hidden (no downed ALLY in reach).
#[test]
fn adjacent_downed_enemy_offers_execute() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // A downed ENEMY (gang 1) one cell diagonally — 8-adjacent.
    spawn_downed(&mut app, 6, 6, 1, None);
    app.update();

    assert!(
        execute_visible(&mut app),
        "an 8-adjacent downed enemy must reveal the Execute button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Execute must reveal the panel root",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
}

/// STABILIZE condition: a selected actor with an 8-adjacent unstabilized downed ALLY (same
/// faction) offers Stabilize — the Stabilize button + the panel root become Visible.
#[test]
fn adjacent_downed_ally_offers_stabilize() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // A downed, not-yet-stabilized ALLY (gang 0) orthogonally adjacent — 8-adjacent.
    spawn_downed(&mut app, 5, 6, 0, Some(false));
    app.update();

    assert!(
        stabilize_visible(&mut app),
        "an 8-adjacent unstabilized downed ally must reveal the Stabilize button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Stabilize must reveal the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
}

/// HIDDEN condition: a selected actor with NO adjacent downed neighbour hides the panel root +
/// the act buttons (no act offered).
#[test]
fn no_adjacent_downed_hides_panel() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // A downed enemy FAR away (not 8-adjacent) — not a candidate.
    spawn_downed(&mut app, 20, 20, 1, None);
    app.update();

    assert!(
        !root_visible(&mut app),
        "no downed neighbour in reach -> the panel root stays hidden",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
    assert_eq!(
        visibility::<OpenDoorButton>(&mut app),
        Some(Visibility::Hidden),
        "the Open Door button is a deferred act and stays hidden",
    );
}

/// REACTIVE / no respawn: after the panel shows for an adjacent downed enemy, moving the actor
/// out of reach hides it again — and the SAME button entities persist (asserted by `Entity` id),
/// proving the system TOGGLES `Visibility` rather than despawning + respawning the scaffold.
#[test]
fn moving_actor_away_hides_panel_without_respawn() {
    let mut app = battle_running_app();
    let actor = spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 6, 6, 1, None);
    app.update();

    // Sanity: the panel is showing, and capture the button entity ids.
    assert!(
        execute_visible(&mut app),
        "sanity: the Execute button is shown"
    );
    let execute_before = single_with::<ExecuteButton>(&mut app);
    let stabilize_before = single_with::<StabilizeButton>(&mut app);
    let root_before = single_with::<ContextualPanelRoot>(&mut app);

    // Move the actor far from the downed enemy (mutate Position in place).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(actor) {
        *pos = at(40, 40);
    }
    app.update();

    assert!(
        !root_visible(&mut app),
        "moving the actor out of reach must hide the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "moving the actor out of reach must hide the Execute button",
    );

    // The SAME entities still exist — a Visibility toggle, not a despawn/respawn.
    assert_eq!(
        single_with::<ExecuteButton>(&mut app),
        execute_before,
        "the Execute button entity must persist (Visibility toggle, not respawn)",
    );
    assert_eq!(
        single_with::<StabilizeButton>(&mut app),
        stabilize_before,
        "the Stabilize button entity must persist (Visibility toggle, not respawn)",
    );
    assert_eq!(
        single_with::<ContextualPanelRoot>(&mut app),
        root_before,
        "the panel root entity must persist (Visibility toggle, not respawn)",
    );
}

// ---------------------------------------------------------------------------------
// Press → intent AC — a contextual button press routes the carried target through the
// REAL GTW-571 per-act seam (button -> generic press router -> the act's buffered queue
// -> the act's generic drain, all the same update).
// ---------------------------------------------------------------------------------

/// PRESS → INTENT: with an Execute target offered (an 8-adjacent downed enemy), pressing the
/// Execute button drives the REAL GTW-571 stack (button -> the generic press router ->
/// `PendingContextualIntents<ExecuteAct>` -> the act's generic drain) to emit exactly one
/// `ExecuteDownedRequested` for the `SelectedShooter` as actor over the carried downed target.
/// The load-bearing assertion is the END message the press produces — the same parity idiom the
/// action-bar end-turn test uses, and strictly stronger than reading the queue.
///
/// Pin-discriminating: dropping the act's press registration (or the offer scan that fills the
/// target) leaves the queue empty and emits zero messages, failing the asserts.
#[test]
fn pressing_execute_emits_execute_downed_requested_for_target() {
    let mut app = battle_running_app();
    add_execute_probe(&mut app);
    let actor = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_downed(&mut app, 6, 6, 1, None);

    // First update: detection reveals the Execute button + fills the target offer.
    app.update();
    assert!(
        execute_visible(&mut app),
        "sanity: the Execute button is offered before the press",
    );
    let Some(execute_btn) = single_with::<ExecuteButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `execute_visible` assert above.
        return;
    };

    // Drive the press, then update: the generic press router pushes the offered target and the
    // act's generic drain (the Press set is ordered before the Drain set) emits
    // ExecuteDownedRequested the SAME update.
    press_ui_button(&mut app, execute_btn);
    app.update();

    let emitted = executes(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Execute with a target offered must emit exactly one ExecuteDownedRequested",
    );
    assert_eq!(emitted[0].actor, actor, "the actor is the SelectedShooter");
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried downed neighbour",
    );
}

/// PRESS → INTENT: with a Stabilize target offered (an 8-adjacent unstabilized downed ALLY),
/// pressing the Stabilize button drives the REAL GTW-571 stack (button -> the generic press
/// router -> `PendingContextualIntents<StabilizeAct>` -> the act's generic drain) to emit exactly
/// one `StabilizeDownedRequested` for the `SelectedShooter` as actor over the carried downed ally,
/// the SAME update (the `pressing_execute_...` mirror over the ALLY arm — the GTW-571 AC-3
/// end-to-end leg for the rewired `press_contextual_button::<StabilizeAct>` instantiation).
///
/// Pin-discriminating: dropping the act's press registration (or the offer scan that fills the
/// target) leaves the queue empty and emits zero messages, failing the asserts.
#[test]
fn pressing_stabilize_emits_stabilize_downed_requested_for_target() {
    let mut app = battle_running_app();
    add_stabilize_probe(&mut app);
    let actor = spawn_actor(&mut app, 5, 5, 0);
    // A downed, not-yet-stabilized ALLY (gang 0) orthogonally adjacent — the Stabilize offer.
    let target = spawn_downed(&mut app, 5, 6, 0, Some(false));

    // First update: detection reveals the Stabilize button + fills the target offer.
    app.update();
    assert!(
        stabilize_visible(&mut app),
        "sanity: the Stabilize button is offered before the press",
    );
    let Some(stabilize_btn) = single_with::<StabilizeButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `stabilize_visible` assert above.
        return;
    };

    // Drive the press, then update: the generic press router pushes the offered target and the
    // act's generic drain emits StabilizeDownedRequested the SAME update.
    press_ui_button(&mut app, stabilize_btn);
    app.update();

    let emitted = stabilizes(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Stabilize with a target offered must emit exactly one StabilizeDownedRequested",
    );
    assert_eq!(emitted[0].actor, actor, "the actor is the SelectedShooter");
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried downed ally",
    );
}
