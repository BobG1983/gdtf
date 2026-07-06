//! GTW-315 OPEN DOOR button: detection + press toggles the door.

use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::OpenDoorButton;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    cover::HeightBand,
    entity::TerrainCell,
    openable::{OpenState, OpenableBlocking},
    prelude::{Cell, CellLevel, Faction, Level, Position},
};
use gdtf_test_utils::{advance_until, press_ui_button};

use super::{actors::*, harness::*};

/// Spawns the OPEN-DOOR actor — a ganger carrying exactly the components the open-door path reads:
/// its [`Position`] + [`Faction`] (detection adjacency) and a full [`Tu`] / [`TuMax`] pool (the
/// sim's `dispatch_open_door` affords + charges the `OpenDoorTu` leaf off it) — at cell `(x, y)` in
/// gang `gang`, and SELECTS it via [`SelectedShooter`] (GTW-315). Returns its entity. A generous TU
/// pool so the sim gate never rejects on affordability.
fn spawn_door_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app
        .world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            gdtf_battle_sim::ganger::Tu::new(100),
            gdtf_battle_sim::ganger::TuMax::new(100),
        ))
        .id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Spawns a CLOSED openable DOOR at cell `(x, y)` — a terrain entity carrying exactly what the
/// GTW-503 openable mechanism (and `setup_battle`) attach for an openable piece that the open-door
/// path reads/flips: its [`TerrainCell`] (the adjacency cell), [`OpenState::Closed`] (the scan
/// offers only a CLOSED door), and an [`OpenableBlocking`] band record (the toggle re-block source).
/// Returns its entity. The optional `state` overrides the closed default so a test can spawn an
/// already-OPEN door (which detection must NOT offer). The detection scan reads `OpenState` +
/// `TerrainCell`; `apply_openable_toggle` reads `OpenState` + `OpenableBlocking`.
fn spawn_door(app: &mut App, x: i32, y: i32, state: OpenState) -> Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            state,
            OpenableBlocking::new(HeightBand::High),
        ))
        .id()
}

/// Reads whether the Open Door button is visible (GTW-315).
fn open_door_visible(app: &mut App) -> bool {
    visibility::<OpenDoorButton>(app) == Some(Visibility::Visible)
}

/// Reads a door's current [`OpenState`], if it still carries one.
fn door_state(app: &App, door: Entity) -> Option<OpenState> {
    app.world().get::<OpenState>(door).copied()
}

// ---------------------------------------------------------------------------------
// GTW-315 — the OPEN DOOR button: detection (an 8-adjacent CLOSED door reveals it, an
// already-open / non-adjacent door does NOT) and press → the door's OpenState toggles
// to Open through the REAL seam + the app-wired sim.
// ---------------------------------------------------------------------------------

/// OPEN-DOOR detection: a selected player actor with an 8-adjacent CLOSED door offers Open Door —
/// the dedicated Open Door button + the panel root become Visible (GTW-315). The button ALWAYS
/// OPENS (a closed door is the only offered target); F4 is PLAYER-ONLY (the selected actor is the
/// player-faction ganger). No ganger acts are offered (no downed / alive neighbour present).
#[test]
fn adjacent_closed_door_offers_open_door() {
    let mut app = battle_running_app();
    spawn_door_actor(&mut app, 5, 5, 0);
    // A CLOSED door one cell diagonally — 8-adjacent.
    spawn_door(&mut app, 6, 6, OpenState::Closed);
    app.update();

    assert!(
        open_door_visible(&mut app),
        "an 8-adjacent CLOSED door must reveal the Open Door button (GTW-315)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Open Door must reveal the panel root",
    );
    // No ganger neighbour present -> the ganger acts stay hidden.
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
    assert!(
        !melee_visible(&mut app),
        "no meleeable neighbour in reach -> the Melee button stays hidden",
    );
    assert!(
        !shove_visible(&mut app),
        "no shovable neighbour in reach -> the Shove button stays hidden",
    );
}

/// OPEN-DOOR detection — the gate is CLOSED + 8-adjacent: an already-OPEN adjacent door and a
/// CLOSED but non-adjacent door each offer NO open (the button + panel root stay hidden, and no
/// stray ganger act is offered). Discriminating: moving the actor next to the CLOSED far door
/// reveals it.
#[test]
fn open_or_non_adjacent_door_does_not_offer_open_door() {
    let mut app = battle_running_app();
    let actor = spawn_door_actor(&mut app, 5, 5, 0);
    // An already-OPEN door 8-adjacent — the button only OPENS, so it is NOT offered.
    spawn_door(&mut app, 5, 6, OpenState::Open);
    // A CLOSED door FAR away (Chebyshev > 1) — not adjacent, so not offered.
    spawn_door(&mut app, 30, 30, OpenState::Closed);
    app.update();

    assert!(
        !open_door_visible(&mut app),
        "an adjacent OPEN door + a non-adjacent CLOSED door offer NO open (button hidden)",
    );
    assert!(
        !root_visible(&mut app),
        "with no contextual act offered the panel root stays hidden",
    );

    // Move the actor next to the FAR closed door — the CLOSED + 8-adjacent gate now passes and the
    // Open Door button reveals (discriminating: the gate is CLOSED + adjacency, not mere presence).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(actor) {
        *pos = at(29, 30);
    }
    app.update();
    assert!(
        open_door_visible(&mut app),
        "moving the actor next to the CLOSED door reveals the Open Door button (discriminating)",
    );
}

/// PRESS → OPEN: with an Open Door target offered (an 8-adjacent CLOSED door), pressing the Open
/// Door button drives the REAL stack (button -> the generic press router ->
/// `PendingContextualIntents<OpenDoorAct>` -> the act's generic drain -> `OpenDoorRequested` ->
/// the app-wired sim `dispatch_open_door` ->
/// `SetOpenable::toggle` -> `apply_openable_toggle`) so the SPECIFIC carried door's `OpenState`
/// flips CLOSED -> Open (GTW-315). Driven THROUGH the button/intent/sim path end to end — never a
/// synthetic `SetOpenable` emit — proving the correct door entity was carried across the seam.
///
/// The door flip settles one frame after the toggle message (the GTW-503 documented one-frame
/// settle: `dispatch_open_door` writes `SetOpenable`, `apply_openable_toggle` flips `OpenState` and
/// removes the blocking components), so the assertion advances a couple of updates.
///
/// Pin-discriminating: dropping the act's press registration (or the offer scan that fills the
/// target) leaves the queue empty, no `OpenDoorRequested` is emitted, and the door stays CLOSED,
/// failing the assert.
#[test]
fn pressing_open_door_toggles_the_door_open() {
    let mut app = battle_running_app();
    spawn_door_actor(&mut app, 5, 5, 0);
    let door = spawn_door(&mut app, 6, 6, OpenState::Closed);

    // First update: detection reveals the Open Door button + fills the door offer.
    app.update();
    assert!(
        open_door_visible(&mut app),
        "sanity: the Open Door button is offered before the press",
    );
    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Closed),
        "sanity: the door is CLOSED before the press",
    );
    let Some(open_door_btn) = single_with::<OpenDoorButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `open_door_visible` assert above.
        return;
    };

    // Drive the press, then advance: the generic press router pushes the offered door, the drain
    // emits OpenDoorRequested + the sim dispatch writes SetOpenable::toggle the SAME update, and
    // apply_openable_toggle flips OpenState one frame later (the GTW-503 one-frame settle).
    press_ui_button(&mut app, open_door_btn);
    let opened = advance_until(
        &mut app,
        |app| door_state(app, door) == Some(OpenState::Open),
        BUDGET,
    );
    assert!(
        opened,
        "pressing Open Door on the carried CLOSED door must toggle its OpenState to Open through \
         the real seam + sim; last was {:?}",
        door_state(&app, door),
    );
}
