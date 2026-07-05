//! GTW-543 ENTER/EXIT EMPLACEMENT buttons: detection, occupancy gating, press.

use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::{EnterEmplacementButton, ExitEmplacementButton};
use gdtf_battle_input::{SelectedShooter, contextual::ContextualActSystems};
use gdtf_battle_sim::{
    Cell, CellLevel, EmplacementOccupant, EmplacementState, Faction, Level, Position,
    acts::ExitEmplacementRequested, entity::TerrainCell,
};
use gdtf_test_utils::{MessageProbe, advance_until, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

// ---------------------------------------------------------------------------------
// GTW-543 — the ENTER / EXIT EMPLACEMENT buttons: detection (an 8-adjacent VACANT
// emplacement reveals Enter; Exit reveals ONLY for the occupant) and press → the
// emplacement's EmplacementState flips VACANT -> Occupied through the REAL seam + sim.
// ---------------------------------------------------------------------------------

/// Spawns the EMPLACEMENT actor — a ganger carrying exactly the components the enter/exit path
/// reads: its [`Position`] + [`Faction`] (detection adjacency + F4 player scope) and a full [`Tu`]
/// / [`TuMax`] pool (the sim's `dispatch_enter_emplacement` / `dispatch_exit_emplacement` afford +
/// charge the emplacement TU leaves off it) — at cell `(x, y)` in gang `gang`, and SELECTS it via
/// [`SelectedShooter`] (GTW-543). Returns its entity. A generous TU pool so the sim gate never
/// rejects on affordability.
fn spawn_emplacement_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app
        .world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            gdtf_battle_sim::Tu::new(100),
            gdtf_battle_sim::TuMax::new(100),
        ))
        .id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Spawns a weapon EMPLACEMENT terrain entity at cell `(x, y)` — carrying exactly what the enter
/// scan + the sim toggle read: its [`TerrainCell`] (the adjacency cell) and an [`EmplacementState`]
/// (the enter scan offers only a VACANT one). When `occupant` is `Some`, it also spawns the
/// `EmplacementState::Occupied` + an [`EmplacementOccupant`] recording that ganger (the exit scan
/// offers the emplacement whose occupant IS the selection). Returns its entity.
fn spawn_emplacement(
    app: &mut App,
    x: i32,
    y: i32,
    state: EmplacementState,
    occupant: Option<Entity>,
) -> Entity {
    let mut entity = app.world_mut().spawn((
        TerrainCell::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
        state,
    ));
    if let Some(occupant) = occupant {
        entity.insert(EmplacementOccupant::new(occupant));
    }
    entity.id()
}

/// Reads whether the Enter Emplacement button is visible (GTW-543).
fn enter_emplacement_visible(app: &mut App) -> bool {
    visibility::<EnterEmplacementButton>(app) == Some(Visibility::Visible)
}

/// Reads whether the Exit Emplacement button is visible (GTW-543).
fn exit_emplacement_visible(app: &mut App) -> bool {
    visibility::<ExitEmplacementButton>(app) == Some(Visibility::Visible)
}

/// Reads an emplacement's current [`EmplacementState`], if it still carries one.
fn emplacement_state(app: &App, emplacement: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(emplacement).copied()
}

/// ENTER detection: a selected player actor with an 8-adjacent VACANT emplacement offers Enter —
/// the dedicated Enter button + the panel root become Visible (GTW-543). The Exit button stays
/// hidden (the selection is not manning anything), and no ganger acts are offered (no neighbour).
#[test]
fn adjacent_vacant_emplacement_offers_enter() {
    let mut app = battle_running_app();
    spawn_emplacement_actor(&mut app, 5, 5, 0);
    // A VACANT emplacement one cell diagonally — 8-adjacent.
    spawn_emplacement(&mut app, 6, 6, EmplacementState::Vacant, None);
    app.update();

    assert!(
        enter_emplacement_visible(&mut app),
        "an 8-adjacent VACANT emplacement must reveal the Enter button (GTW-543)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Enter must reveal the panel root",
    );
    assert!(
        !exit_emplacement_visible(&mut app),
        "the selection is not manning an emplacement -> the Exit button stays hidden",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !shove_visible(&mut app),
        "no shovable neighbour in reach -> the Shove button stays hidden",
    );
}

/// ENTER detection — the gate is VACANT + 8-adjacent: an OCCUPIED adjacent emplacement and a VACANT
/// but non-adjacent emplacement each offer NO enter (the button + panel root stay hidden).
/// Discriminating: moving the actor next to the VACANT far emplacement reveals Enter.
#[test]
fn occupied_or_non_adjacent_emplacement_does_not_offer_enter() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    // An already-OCCUPIED emplacement 8-adjacent (occupied by SOME other ganger) — the enter scan
    // offers only a VACANT emplacement, so it is NOT offered. A distinct occupant entity.
    let other = app.world_mut().spawn_empty().id();
    spawn_emplacement(&mut app, 5, 6, EmplacementState::Occupied, Some(other));
    // A VACANT emplacement FAR away (Chebyshev > 1) — not adjacent, so not offered.
    spawn_emplacement(&mut app, 30, 30, EmplacementState::Vacant, None);
    app.update();

    assert!(
        !enter_emplacement_visible(&mut app),
        "an adjacent OCCUPIED emplacement + a non-adjacent VACANT one offer NO enter (button hidden)",
    );

    // Move the actor next to the FAR vacant emplacement — the VACANT + 8-adjacent gate now passes
    // and the Enter button reveals (discriminating: the gate is VACANT + adjacency, not presence).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(actor) {
        *pos = at(29, 30);
    }
    app.update();
    assert!(
        enter_emplacement_visible(&mut app),
        "moving the actor next to the VACANT emplacement reveals the Enter button (discriminating)",
    );
}

/// EXIT detection: the Exit button reveals ONLY for the OCCUPANT. With the selection recorded as
/// an emplacement's [`EmplacementOccupant`], Exit is offered (no adjacency needed — the occupant is
/// on the mount); an emplacement occupied by SOMEONE ELSE offers the selection no Exit.
#[test]
fn exit_offered_only_to_the_occupant() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    // The selection IS the recorded occupant of this emplacement (co-located at the actor cell).
    spawn_emplacement(&mut app, 5, 5, EmplacementState::Occupied, Some(actor));
    app.update();

    assert!(
        exit_emplacement_visible(&mut app),
        "the selection manning an emplacement must reveal the Exit button (GTW-543)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Exit must reveal the panel root",
    );
    // Occupied by the selection -> the Enter offer (which needs a VACANT emplacement) is hidden.
    assert!(
        !enter_emplacement_visible(&mut app),
        "the only emplacement is OCCUPIED (by the selection) -> the Enter button stays hidden",
    );

    // Now record a DIFFERENT ganger as the occupant — the selection is no longer the occupant, so
    // Exit is no longer offered (discriminating: Exit is offered to the OCCUPANT only).
    let other = app.world_mut().spawn_empty().id();
    let emplacements = all_with::<EmplacementState>(&mut app);
    if let [emplacement] = emplacements.as_slice() {
        app.world_mut()
            .entity_mut(*emplacement)
            .insert(EmplacementOccupant::new(other));
    }
    app.update();
    assert!(
        !exit_emplacement_visible(&mut app),
        "an emplacement occupied by SOMEONE ELSE offers the selection no Exit (occupant-only)",
    );
}

/// PRESS → ENTER: with an Enter target offered (an 8-adjacent VACANT emplacement), pressing the
/// Enter button drives the REAL stack (button -> the generic press router ->
/// `PendingContextualIntents<EnterEmplacementAct>` -> the
/// ONE `dispatch_act_intents` drain -> `EnterEmplacementRequested` -> the app-wired sim
/// `dispatch_enter_emplacement` -> `SetEmplacement::occupy` -> `apply_emplacement_toggle`) so the
/// SPECIFIC carried emplacement's `EmplacementState` flips VACANT -> Occupied (GTW-543). Driven
/// THROUGH the button/intent/sim path end to end — never a synthetic `SetEmplacement` emit —
/// proving the correct emplacement entity was carried across the seam.
///
/// The occupy settles a couple frames after the toggle message (the one-frame settle:
/// `dispatch_enter_emplacement` writes `SetEmplacement`, `apply_emplacement_toggle` flips
/// `EmplacementState`), so the assertion advances a few updates.
///
/// Pin-discriminating: dropping the act's press registration (or the offer scan that
/// fills the target) leaves the queue empty, no `EnterEmplacementRequested` is emitted, and the
/// emplacement stays VACANT, failing the assert.
#[test]
fn pressing_enter_mans_the_emplacement() {
    let mut app = battle_running_app();
    spawn_emplacement_actor(&mut app, 5, 5, 0);
    let emplacement = spawn_emplacement(&mut app, 6, 6, EmplacementState::Vacant, None);

    // First update: detection reveals the Enter button + fills the emplacement offer.
    app.update();
    assert!(
        enter_emplacement_visible(&mut app),
        "sanity: the Enter button is offered before the press",
    );
    assert_eq!(
        emplacement_state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "sanity: the emplacement is VACANT before the press",
    );
    let Some(enter_btn) = single_with::<EnterEmplacementButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `enter_emplacement_visible` assert.
        return;
    };

    // Drive the press, then advance: the generic press router pushes the offered emplacement,
    // the drain emits EnterEmplacementRequested + the sim dispatch writes SetEmplacement::occupy the
    // SAME update, and apply_emplacement_toggle flips EmplacementState one frame later.
    press_ui_button(&mut app, enter_btn);
    let manned = advance_until(
        &mut app,
        |app| emplacement_state(app, emplacement) == Some(EmplacementState::Occupied),
        BUDGET,
    );
    assert!(
        manned,
        "pressing Enter on the carried VACANT emplacement must flip its EmplacementState to \
         Occupied through the real seam + sim; last was {:?}",
        emplacement_state(&app, emplacement),
    );

    // The occupant is recorded as the acting selection (proves the carried emplacement was manned by
    // this actor, not merely flipped).
    let occupant = app
        .world()
        .get::<EmplacementOccupant>(emplacement)
        .map(|occupant| **occupant);
    let selected = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|selection| **selection);
    assert_eq!(
        occupant, selected,
        "the recorded EmplacementOccupant is the acting selection",
    );
}

/// Adds the [`ExitEmplacementRequested`] probe — the generic GTW-576 `MessageProbe<M>` with its drain
/// at the ORIGINAL observation point (Update, AFTER the GTW-571 contextual drain set),
/// so it pins the DRAIN's same-update emission, not any later sim re-emission.
fn add_exit_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ExitEmplacementRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ExitEmplacementRequested>.after(ContextualActSystems::Drain),
    );
}

/// The collected [`ExitEmplacementRequested`] messages.
fn exit_requests(app: &App) -> Vec<ExitEmplacementRequested> {
    probed::<ExitEmplacementRequested>(app)
}

/// PRESS → INTENT: with an Exit target offered (the selection recorded as an emplacement's
/// [`EmplacementOccupant`]), pressing the Exit button drives the REAL GTW-571 stack (button ->
/// the generic press router -> `PendingContextualIntents<ExitEmplacementAct>` -> the act's
/// generic drain) to emit exactly one `ExitEmplacementRequested` for the `SelectedShooter` as
/// actor dismounting the carried emplacement, the SAME update (the GTW-571 AC-3 end-to-end leg
/// for the rewired `press_contextual_button::<ExitEmplacementAct>` instantiation).
///
/// Pin-discriminating: dropping the act's press registration (or the occupant scan that fills the
/// target) leaves the queue empty and emits zero messages, failing the asserts.
#[test]
fn pressing_exit_emits_exit_emplacement_requested_for_manned_mount() {
    let mut app = battle_running_app();
    add_exit_probe(&mut app);
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    // The selection IS the recorded occupant of this co-located emplacement — the Exit offer.
    let emplacement = spawn_emplacement(&mut app, 5, 5, EmplacementState::Occupied, Some(actor));

    // First update: detection reveals the Exit button + fills the emplacement offer.
    app.update();
    assert!(
        exit_emplacement_visible(&mut app),
        "sanity: the Exit button is offered before the press",
    );
    let Some(exit_btn) = single_with::<ExitEmplacementButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `exit_emplacement_visible` assert.
        return;
    };

    // Drive the press, then update: the generic press router pushes the offered emplacement and
    // the act's generic drain emits ExitEmplacementRequested the SAME update.
    press_ui_button(&mut app, exit_btn);
    app.update();

    let emitted = exit_requests(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Exit while manning must emit exactly one ExitEmplacementRequested",
    );
    assert_eq!(emitted[0].actor, actor, "the actor is the SelectedShooter");
    assert_eq!(
        emitted[0].emplacement, emplacement,
        "the emplacement is the carried manned mount",
    );
}
