//! GTW-546 THROW button: arc-weapon + hovered-cell detection + press.

use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::ThrowGrenadeButton;
use gdtf_battle_input::{InspectTarget, SelectedShooter, contextual::ContextualActSystems};
use gdtf_battle_sim::{
    acts::ThrowGrenadeRequested,
    prelude::{Cell, CellLevel, Faction, Level},
    weapon::{TrajectoryStyle, WieldedBy},
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

// ---------------------------------------------------------------------------------
// GTW-546 — the THROW button: detection (a selection wielding a TrajectoryStyle::Arc
// weapon + a hovered target cell reveals it, a Straight-weapon / no-hover selection does
// NOT) and press → ThrowGrenadeRequested for the hovered cell through the REAL seam.
// ---------------------------------------------------------------------------------

/// Spawns the THROW actor — a ganger carrying the components the throw path reads (its
/// [`Position`] + [`Faction`]) — at cell `(x, y)` in gang `gang`, RELATES a weapon entity carrying
/// the given [`TrajectoryStyle`] to it (via [`WieldedBy`] — the sim's `Wields` relationship, which
/// the throw scan resolves through), and SELECTS the ganger via [`SelectedShooter`]. Returns the
/// ganger entity. The related weapon carries NO `MeleeWeapon` marker, so the throw scan's
/// `ranged_weapon` resolution finds it (GTW-546).
fn spawn_throw_actor(
    app: &mut App,
    x: i32,
    y: i32,
    gang: u8,
    trajectory: TrajectoryStyle,
) -> Entity {
    let actor = app.world_mut().spawn((at(x, y), Faction::new(gang))).id();
    // A ranged weapon entity wielded by the actor, carrying the trajectory style the scan reads.
    app.world_mut().spawn((WieldedBy::new(actor), trajectory));
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Seeds the [`InspectTarget`] resource's live hovered cell to `(x, y)` on level 0 — the BLIND
/// throw's target cell (the cursor-over-cell the picker would write) (GTW-546).
fn hover_cell(app: &mut App, x: i32, y: i32) {
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(CellLevel::new(
            Cell::new(x, y),
            Level::new(0),
        ))));
}

/// Reads whether the Throw button is visible (GTW-546).
fn throw_visible(app: &mut App) -> bool {
    visibility::<ThrowGrenadeButton>(app) == Some(Visibility::Visible)
}

/// Adds the [`ThrowGrenadeRequested`] probe — the generic GTW-576 `MessageProbe<M>` with its drain
/// at the ORIGINAL observation point (Update, AFTER the GTW-571 contextual drain set),
/// so it pins the DRAIN's same-update emission, not any later sim re-emission.
fn add_throw_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ThrowGrenadeRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ThrowGrenadeRequested>.after(ContextualActSystems::Drain),
    );
}

/// The collected [`ThrowGrenadeRequested`] messages.
fn throws(app: &App) -> Vec<ThrowGrenadeRequested> {
    probed::<ThrowGrenadeRequested>(app)
}

/// THROW detection: a selected actor wielding a [`TrajectoryStyle::Arc`] weapon with a target cell
/// HOVERED offers Throw — the dedicated Throw button + the panel root become Visible (GTW-546). A
/// BLIND lob: no adjacency / LOS / neighbour is required (no downed / alive ganger present), only an
/// `Arc` weapon and a hovered cell.
#[test]
fn arc_weapon_and_hovered_cell_offers_throw() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    hover_cell(&mut app, 15, 15);
    app.update();

    assert!(
        throw_visible(&mut app),
        "an Arc weapon + a hovered target cell must reveal the Throw button (GTW-546)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Throw must reveal the panel root",
    );
    // No neighbour present -> the ganger acts stay hidden.
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !shove_visible(&mut app),
        "no shovable neighbour in reach -> the Shove button stays hidden",
    );
}

/// THROW detection — the gate is an `Arc` weapon + a hovered cell: a [`TrajectoryStyle::Straight`]
/// weapon offers NO throw (the button stays hidden) even with a cell hovered, and an `Arc` weapon
/// with NOTHING hovered offers NO throw. Discriminating: an `Arc` weapon + a hovered cell reveals it.
#[test]
fn straight_weapon_or_no_hover_does_not_offer_throw() {
    let mut app = battle_running_app();
    // A Straight-weapon actor with a cell hovered — a Straight weapon never lobs, so NO throw.
    spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Straight);
    hover_cell(&mut app, 15, 15);
    app.update();
    assert!(
        !throw_visible(&mut app),
        "a Straight-trajectory weapon offers NO throw even with a cell hovered (button hidden)",
    );

    // Swap to an Arc weapon but clear the hover — no target cell, so still NO throw.
    let actor = spawn_throw_actor(&mut app, 6, 6, 0, TrajectoryStyle::Arc);
    app.world_mut().insert_resource(InspectTarget::new(None));
    app.update();
    assert!(
        !throw_visible(&mut app),
        "an Arc weapon with NOTHING hovered offers NO throw (no target cell, button hidden)",
    );

    // Now hover a cell — the SAME Arc actor reveals the Throw button (discriminating: the gate is
    // an Arc weapon + a hovered cell, not mere presence).
    let _ = actor;
    hover_cell(&mut app, 20, 20);
    app.update();
    assert!(
        throw_visible(&mut app),
        "an Arc weapon + a hovered cell reveals the Throw button (discriminating)",
    );
}

/// PRESS → INTENT: with a Throw target offered (an `Arc` weapon + a hovered cell), pressing the
/// Throw button drives the REAL stack (button -> the generic press router ->
/// `PendingContextualIntents<ThrowGrenadeAct>` -> the act's generic drain) to emit exactly one
/// `ThrowGrenadeRequested` for the `SelectedShooter` as thrower over the carried HOVERED cell
/// (GTW-546) — driven THROUGH the button/press path, NOT a synthetic emit.
///
/// Pin-discriminating: dropping the act's press registration (or the offer scan that fills the
/// target) leaves the queue empty and emits zero messages, failing the asserts.
#[test]
fn pressing_throw_emits_throw_grenade_requested_for_hovered_cell() {
    let mut app = battle_running_app();
    add_throw_probe(&mut app);
    let thrower = spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    let target = CellLevel::new(Cell::new(15, 15), Level::new(0));
    hover_cell(&mut app, 15, 15);

    // First update: detection reveals the Throw button + fills the target offer.
    app.update();
    assert!(
        throw_visible(&mut app),
        "sanity: the Throw button is offered before the press",
    );
    let Some(throw_btn) = single_with::<ThrowGrenadeButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `throw_visible` assert above.
        return;
    };

    // Re-seed the hovered cell (the headless picker overwrites InspectTarget each update with no
    // live cursor), then drive the press + update: the offer scan (the Offer set precedes the
    // picker) re-fills the throw target from this hover, the generic press router pushes it, and
    // the act's generic drain emits ThrowGrenadeRequested the SAME update.
    hover_cell(&mut app, 15, 15);
    press_ui_button(&mut app, throw_btn);
    app.update();

    let emitted = throws(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Throw with a target offered must emit exactly one ThrowGrenadeRequested",
    );
    assert_eq!(
        emitted[0].thrower, thrower,
        "the thrower is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried HOVERED cell (the blind lob's aim)",
    );
}
