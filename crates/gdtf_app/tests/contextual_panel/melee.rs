//! GTW-507 MELEE button: detection + press.

use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::MeleeButton;
use gdtf_battle_input::{SelectedShooter, contextual::ContextualActSystems};
use gdtf_battle_sim::{
    acts::{MeleeRequested, MeleeTarget},
    ganger::Facing,
    prelude::{Direction, Faction, Position, Stance, StanceKind},
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

/// Spawns the MELEE actor — a ganger carrying exactly the components the melee detection reads
/// off the selection (its [`Position`] + [`Faction`] + [`Stance`] + [`Facing`], for the LOS
/// observer eye) — at cell `(x, y)` in gang `gang`, and SELECTS it (GTW-507). Returns its entity.
fn spawn_melee_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app
        .world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            Stance::new(StanceKind::Standing),
            Facing::new(Direction::East),
        ))
        .id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Adds the [`MeleeRequested`] probe — the generic GTW-576 `MessageProbe<M>` with its drain
/// at the ORIGINAL observation point (Update, AFTER the GTW-571 contextual drain set),
/// so it pins the DRAIN's same-update emission, not any later sim re-emission.
fn add_melee_probe(app: &mut App) {
    app.init_resource::<MessageProbe<MeleeRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<MeleeRequested>.after(ContextualActSystems::Drain),
    );
}

/// The collected [`MeleeRequested`] messages.
fn melees(app: &App) -> Vec<MeleeRequested> {
    probed::<MeleeRequested>(app)
}

// ---------------------------------------------------------------------------------
// GTW-507 — the MELEE button: detection (alive + 8-adjacent + LOS enemy reveals it) and
// press → MeleeRequested through the REAL seam.
// ---------------------------------------------------------------------------------

/// MELEE detection: a selected actor with an 8-adjacent, ALIVE, in-LOS ENEMY offers Melee — the
/// dedicated Melee button + the panel root become Visible (GTW-507). A STRONGER gate than
/// Execute's downed-adjacency: the enemy is ALIVE (not downed), and the LOS gate (reusing the
/// sim's `has_los` over the live battle grids) clears the open adjacent cell.
#[test]
fn adjacent_alive_enemy_in_los_offers_melee() {
    let mut app = battle_running_app();
    spawn_melee_actor(&mut app, 5, 5, 0);
    // An ALIVE ENEMY (gang 1) one cell diagonally — 8-adjacent, clear LOS (no cover between).
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();

    assert!(
        melee_visible(&mut app),
        "an 8-adjacent alive enemy in LOS must reveal the Melee button (GTW-507)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Melee must reveal the panel root",
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

/// MELEE detection — the LIVE gate is stronger than mere adjacency: an alive enemy that is NOT
/// 8-adjacent does NOT offer Melee (the button stays hidden), and an alive ALLY (same faction)
/// never offers Melee. Discriminating: the same enemy moved INTO reach reveals it.
#[test]
fn non_adjacent_or_ally_does_not_offer_melee() {
    let mut app = battle_running_app();
    spawn_melee_actor(&mut app, 5, 5, 0);
    // An alive ENEMY far away (Chebyshev > 1) — not a melee candidate.
    let enemy = spawn_alive_enemy(&mut app, 20, 20, 1);
    // An alive ALLY 8-adjacent — same faction, never a melee target.
    spawn_alive_enemy(&mut app, 5, 6, 0);
    app.update();

    assert!(
        !melee_visible(&mut app),
        "a non-adjacent enemy + an adjacent ALLY offer NO melee (the button stays hidden)",
    );

    // Move the enemy INTO 8-adjacency — the SAME enemy now reveals the Melee button (the gate is
    // adjacency + alive + opposing + LOS, not mere presence).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(enemy) {
        *pos = at(6, 6);
    }
    app.update();
    assert!(
        melee_visible(&mut app),
        "moving the alive enemy into 8-adjacency reveals the Melee button (discriminating)",
    );
}

/// PRESS → INTENT: with a Melee target offered (an 8-adjacent alive in-LOS enemy), pressing the
/// Melee button drives the REAL GTW-571 stack (button -> the generic press router ->
/// `PendingContextualIntents<MeleeAct>` carrying a `MeleeTarget::Ganger` -> the act's generic
/// drain) to emit exactly one `MeleeRequested` for the `SelectedShooter` as attacker over the
/// carried target (GTW-507) — driven THROUGH the button/press path, NOT a synthetic
/// `MeleeRequested` emit.
///
/// Pin-discriminating: dropping the act's press registration (or the offer scan that fills the
/// target) leaves the queue empty and emits zero messages, failing the asserts.
#[test]
fn pressing_melee_emits_melee_requested_for_target() {
    let mut app = battle_running_app();
    add_melee_probe(&mut app);
    let attacker = spawn_melee_actor(&mut app, 5, 5, 0);
    let target = spawn_alive_enemy(&mut app, 6, 6, 1);

    // First update: detection reveals the Melee button + fills the target offer.
    app.update();
    assert!(
        melee_visible(&mut app),
        "sanity: the Melee button is offered before the press",
    );
    let Some(melee_btn) = single_with::<MeleeButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `melee_visible` assert above.
        return;
    };

    // Drive the press, then update: the generic press router pushes the offered ganger target
    // and the act's generic drain emits MeleeRequested the SAME update.
    press_ui_button(&mut app, melee_btn);
    app.update();

    let emitted = melees(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Melee with a target offered must emit exactly one MeleeRequested",
    );
    assert_eq!(
        emitted[0].attacker, attacker,
        "the attacker is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target,
        MeleeTarget::Ganger(target),
        "the target is the carried opposing neighbour (the ganger melee form)",
    );
}
