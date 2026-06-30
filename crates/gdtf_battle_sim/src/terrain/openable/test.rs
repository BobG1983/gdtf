//! GTW-503 (C5) — the openable toggle drives the REAL GTW-501 / GTW-502 path + vision
//! surfaces through the live systems.
//!
//! Drives [`apply_openable_toggle`](super::apply_openable_toggle) (via
//! [`OpenableTogglePlugin`](super::OpenableTogglePlugin)) AND the GTW-501 / GTW-502
//! projection (via
//! [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)) in a
//! headless `App` (`MinimalPlugins`, no window/renderer — the `path_blocking_test` /
//! `vision_blocking_test` precedent), driving the REAL [`SetOpenable`](super::SetOpenable)
//! message + `app.update()`-ticking so `Added` / `RemovedComponents` fire for real. The
//! tests assert STRUCTURAL facts (path-blocked / vision-occluded), never brittle magnitudes,
//! and exercise the BlockingInProgress-free maintenance plugin directly (no battle gate) —
//! mirroring the landed GTW-501/502 projection tests.
//!
//! ## One-frame settle
//!
//! `apply_openable_toggle` queues the component add/remove via `Commands` (deferred), so the
//! GTW-501 / GTW-502 projection observes the change on the FOLLOWING tick. Each toggle is
//! therefore followed by TWO `app.update()`s: the first applies the toggle's deferred
//! `Commands`, the second lets the projection's `Added` / `RemovedComponents` re-sync the
//! surfaces. This is the documented, deterministic settle (`toggle.rs` module docs / C3) —
//! it is asserted, not assumed.

use bevy::prelude::{App, Entity, MinimalPlugins};

use crate::{
    cover::HeightBand,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    surface::SurfaceGrid,
    terrain::{
        entity::{BlocksPathfinding, BlocksVision, TerrainCell},
        openable::{OpenState, OpenableBlocking, OpenableTogglePlugin, SetOpenable},
    },
};

/// A `(x, y, level)` cell-key helper.
fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// Build the headless app: `MinimalPlugins` + the grid resources + the GTW-501/502
/// maintenance plugin (owns `project_path_blocking` / `project_vision_blocking`) + the
/// GTW-503 toggle plugin (owns `apply_openable_toggle`). The `SurfaceGrid` is seeded because
/// the maintenance plugin's `sync_destroyed_slab` reads it `ResMut`.
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app.add_plugins(OpenableTogglePlugin);
    app
}

/// Spawn a CLOSED openable terrain entity at `at` occluding vision at `band` when closed —
/// exactly what `setup_battle` attaches for an Openable piece (`OpenState::Closed` +
/// `OpenableBlocking` + the forced closed blocking pair). Returns its `Entity`.
fn spawn_closed_openable(app: &mut App, at: CellLevel, band: HeightBand) -> Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(at),
            OpenState::Closed,
            OpenableBlocking::new(band),
            BlocksPathfinding,
            BlocksVision::new(band),
        ))
        .id()
}

/// Whether the grid reports `at` as PATH-blocked — `None` if the grid resource is absent
/// (kept `Option` so the test never `unwrap`s; the restriction lints fire in tests too).
fn path_blocked(app: &App, at: CellLevel) -> Option<bool> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .map(|g| g.is_path_blocked(&at))
}

/// The grid's vision-occluder band at `at` — `None` if absent OR the cell is not occluding.
fn occluder_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.vision_occluder_at(&at))
}

/// The entity's current [`OpenState`], if it still carries one.
fn open_state(app: &App, entity: Entity) -> Option<OpenState> {
    app.world().get::<OpenState>(entity).copied()
}

/// Send a [`SetOpenable`] then settle the toggle + projection (two ticks — the one-frame
/// settle: tick 1 applies the toggle's deferred `Commands`, tick 2 lets the GTW-501/502
/// projection re-sync the surfaces).
fn toggle_and_settle(app: &mut App, request: SetOpenable) {
    app.world_mut().write_message(request);
    app.update();
    app.update();
}

/// C5(a) + C5(b) + C5(c): a CLOSED Openable Wall/Cover (door) blocks path AND occludes `LoS`;
/// toggling OPEN routes path through AND passes `LoS` (the discriminating flip); toggling CLOSED
/// re-blocks both. The full open ⇄ closed cycle through the REAL toggle + GTW-501/502 systems.
#[test]
fn closed_door_blocks_both_open_clears_both_close_reblocks() {
    let at = key(4, 4, 0);
    let mut app = headless_app();
    let door = spawn_closed_openable(&mut app, at, HeightBand::High);

    // Settle the spawn-inserted blocking pair (Added fires on the first tick's projection).
    app.update();
    assert_eq!(
        open_state(&app, door),
        Some(OpenState::Closed),
        "the door spawns Closed (C1 default)",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "C5(a): a CLOSED Openable blocks the path",
    );
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "C5(a): a CLOSED Openable occludes vision (at its closed band)",
    );

    // Toggle OPEN — the discriminating flip: path routes through AND LoS passes.
    toggle_and_settle(&mut app, SetOpenable::open(door));
    assert_eq!(
        open_state(&app, door),
        Some(OpenState::Open),
        "the toggle flipped the door to Open",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "C5(b): an OPEN Openable no longer blocks the path (the discriminating flip)",
    );
    assert_eq!(
        occluder_band(&app, at),
        None,
        "C5(b): an OPEN Openable no longer occludes vision (the discriminating flip)",
    );

    // Toggle CLOSED again — both surfaces re-block.
    toggle_and_settle(&mut app, SetOpenable::close(door));
    assert_eq!(
        open_state(&app, door),
        Some(OpenState::Closed),
        "the toggle flipped the door back to Closed",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "C5(c): toggling CLOSED re-blocks the path",
    );
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "C5(c): toggling CLOSED re-occludes vision",
    );
}

/// C5(d): an Openable SLAB (a kind that does NOT block path or occlude vision by default)
/// STILL blocks BOTH when closed — the C2 case the GTW-501/502 kind-defaults do not cover.
/// The slab's closed band is `HeightBand::High` (it spans the storey). Toggling OPEN clears
/// both, exactly as for a wall door.
#[test]
fn closed_openable_slab_blocks_both_despite_kind_default() {
    let at = key(6, 6, 1);
    let mut app = headless_app();
    // A slab hatch: setup_battle bands a closed openable slab High.
    let hatch = spawn_closed_openable(&mut app, at, HeightBand::High);

    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "C5(d): a CLOSED Openable SLAB blocks the path even though a slab does not by default",
    );
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "C5(d): a CLOSED Openable SLAB occludes vision even though a slab does not by default",
    );

    // Opening the hatch clears both (path + vision).
    toggle_and_settle(&mut app, SetOpenable::open(hatch));
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "C5(d): an OPEN Openable SLAB no longer blocks the path",
    );
    assert_eq!(
        occluder_band(&app, at),
        None,
        "C5(d): an OPEN Openable SLAB no longer occludes vision",
    );
}

/// C5(c) re-tune mirror: the closed band the toggle RE-INSERTS is the one recorded in
/// `OpenableBlocking` (not a hard-coded band) — a Mid-banded door re-blocks at Mid after an
/// open/close cycle. The band record survives the open state (it is NOT removed on open).
#[test]
fn close_reinserts_the_recorded_band() {
    let at = key(2, 8, 0);
    let mut app = headless_app();
    let door = spawn_closed_openable(&mut app, at, HeightBand::Mid);
    app.update();
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::Mid),
        "the door occludes at its recorded Mid band when closed",
    );

    toggle_and_settle(&mut app, SetOpenable::open(door));
    assert_eq!(occluder_band(&app, at), None, "open clears the occluder");

    toggle_and_settle(&mut app, SetOpenable::close(door));
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::Mid),
        "C5: closing RE-INSERTS BlocksVision at the OpenableBlocking-recorded Mid band",
    );
}

/// The toggle is idempotent: re-sending `Open` for an already-open door is a no-op (no spurious
/// re-block on a subsequent close path), and a `SetOpenable` for a non-openable / despawned
/// entity is skipped panic-free.
#[test]
fn toggle_is_idempotent_and_panic_free_on_non_openable() {
    let at = key(1, 1, 0);
    let mut app = headless_app();
    let door = spawn_closed_openable(&mut app, at, HeightBand::High);
    app.update();

    // Open, then re-send Open: still open, path still clear (idempotent — no flip-flop).
    toggle_and_settle(&mut app, SetOpenable::open(door));
    toggle_and_settle(&mut app, SetOpenable::open(door));
    assert_eq!(
        open_state(&app, door),
        Some(OpenState::Open),
        "re-opening an open door stays Open (idempotent)",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "re-opening an open door keeps the path clear",
    );

    // A SetOpenable for a plain (non-openable) entity is skipped panic-free.
    let plain: Entity = app.world_mut().spawn(TerrainCell::new(key(9, 9, 0))).id();
    toggle_and_settle(&mut app, SetOpenable::open(plain));
    assert!(
        open_state(&app, plain).is_none(),
        "a non-openable entity gains no OpenState from a stray SetOpenable (panic-free skip)",
    );
}

/// C5(e): the toggle is DETERMINISTIC — the same spawn + toggle sequence yields the same
/// path/vision surface every run (two independent apps agree). No RNG, no frame-order
/// dependence.
#[test]
fn toggle_is_deterministic() {
    let at = key(3, 3, 0);
    let run = || {
        let mut app = headless_app();
        let door = spawn_closed_openable(&mut app, at, HeightBand::High);
        app.update();
        toggle_and_settle(&mut app, SetOpenable::open(door));
        toggle_and_settle(&mut app, SetOpenable::close(door));
        (path_blocked(&app, at), occluder_band(&app, at))
    };
    assert_eq!(
        run(),
        run(),
        "C5(e): identical spawn+toggle sequences yield identical surfaces (deterministic)",
    );
    assert_eq!(
        run(),
        (Some(true), Some(HeightBand::High)),
        "C5(e): the settled end state is Closed → blocks both",
    );
}
