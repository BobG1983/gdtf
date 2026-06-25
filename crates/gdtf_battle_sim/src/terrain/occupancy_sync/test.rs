use bevy::prelude::{App, Entity, MinimalPlugins};

use super::*;
use crate::{
    clearance::silhouette_band,
    cover::HeightBand,
    ganger::{LifeState, Position, Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
};

fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// Build a headless app: `MinimalPlugins` (no window / renderer — C1), the
/// full 60×60×8 [`OccupancyGrid`] resource, the [`SurfaceGrid`] (GTW-365 —
/// `sync_destroyed_slab` reads it `ResMut`), and the maintenance plugin (which
/// registers the [`CoverDestroyed`] + [`SlabDestroyed`] messages + the four systems).
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    // GTW-365: `sync_destroyed_slab` reads `ResMut<SurfaceGrid>` — seed it so the plugin's
    // four-system band validates (these occupancy tests never destroy a slab, but the
    // system's param must resolve).
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

/// Read the grid resource out of the app world for assertions — `Option` so the
/// test never `unwrap`s (the restriction lints fire in tests too).
fn grid_occupant(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&at))
}

/// Read the published occupant band at `at` — `None` if the grid is absent OR no
/// band is published there (the two collapse for the assertions below, which only
/// care about the band's presence/value at a slot known to exist).
fn grid_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&at))
}

fn cover_destroyed(app: &App, at: CellLevel) -> Option<bool> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .map(|g| g.is_cover_destroyed(&at))
}

/// C9(a) — a ganger MOVES: one tick after mutating [`Position`], the OLD slot
/// is cleared and the NEW slot is marked, WITHOUT any full-grid-rebuild call.
///
/// Spawns a ganger at an initial cell, ticks once (initial placement marks the
/// start slot via the first-run `Changed` semantics), then mutates `Position`
/// to a new cell and ticks again. Asserts the start slot is now empty and the
/// new slot holds the entity — the in-place clear-old + mark-new of C3. The
/// grid is only ever maintained via the systems; `build_from_occupancy_input`
/// is never called.
#[test]
fn moved_ganger_clears_old_slot_and_marks_new() {
    let mut app = headless_app();
    let start = key(5, 6, 0);
    let dest = key(9, 2, 1);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(start),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    // First tick: initial placement (Position reads as Changed on first run).
    app.update();
    assert_eq!(
        grid_occupant(&app, start),
        Some(ganger),
        "initial placement must mark the start slot",
    );
    // GTW-304 — the occupant's silhouette band is published TOGETHER with the
    // occupant (a standing ganger presents the HIGH band).
    assert_eq!(
        grid_band(&app, start),
        Some(HeightBand::High),
        "initial placement must publish the stance-derived band (standing → HIGH)",
    );

    // Move it: mutate Position, then tick once.
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, start),
        None,
        "the OLD slot must be cleared after a move (C3)",
    );
    assert_eq!(
        grid_occupant(&app, dest),
        Some(ganger),
        "the NEW slot must be marked after a move (C3)",
    );
    // The band moves WITH the occupant: cleared from the old slot, published at new.
    assert_eq!(
        grid_band(&app, start),
        None,
        "the OLD slot's band must be cleared after a move (GTW-304)",
    );
    assert_eq!(
        grid_band(&app, dest),
        Some(HeightBand::High),
        "the NEW slot must carry the band after a move (GTW-304)",
    );
}

/// GTW-304 — the published occupant band tracks the ganger's STANCE: a kneeling
/// ganger presents the MID band, a prone ganger the LOW band, and re-posing in
/// place re-publishes the band at the (unchanged) current slot.
#[test]
fn occupant_band_tracks_stance() {
    let mut app = headless_app();
    let at = key(11, 4, 0);

    // Spawn KNEELING — the band must be MID.
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Crouching),
            LifeState::Alive,
        ))
        .id();
    app.update();
    assert_eq!(
        grid_band(&app, at),
        Some(silhouette_band(StanceKind::Crouching)),
        "a kneeling ganger publishes the MID silhouette band",
    );
    assert_eq!(grid_band(&app, at), Some(HeightBand::Mid));

    // Re-pose to PRONE in place (no move) — the band must re-publish as LOW.
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();
    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "the occupant stays put on an in-place re-pose",
    );
    assert_eq!(
        grid_band(&app, at),
        Some(HeightBand::Low),
        "re-posing prone re-publishes the LOW band at the unchanged slot (GTW-304)",
    );
}

/// GTW-304 — a ganger going DOWN clears its band along with its occupant marker
/// (occupant and band stay consistent — no slot left banded but un-occupied).
#[test]
fn dead_ganger_clears_its_band() {
    let mut app = headless_app();
    let at = key(14, 15, 1);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();
    app.update();
    assert_eq!(
        grid_band(&app, at),
        Some(HeightBand::High),
        "the alive ganger's band is published",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Dead;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        None,
        "a dead ganger's occupant slot must be cleared (C4)",
    );
    assert_eq!(
        grid_band(&app, at),
        None,
        "a dead ganger's band must be cleared too (GTW-304)",
    );
}

/// C9(b) — a ganger DIES: flipping [`LifeState`] to [`LifeState::Dead`] and
/// ticking once clears its occupant slot.
///
/// Spawns + places a ganger, then flips its `LifeState` to `Dead` and ticks.
/// The slot it occupied must be freed (C4). In place — no rebuild.
#[test]
fn dead_ganger_clears_its_slot() {
    let mut app = headless_app();
    let at = key(12, 13, 2);

    let ganger = app
        .world_mut()
        .spawn((Position::new(at), LifeState::Alive))
        .id();
    app.update();
    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "the ganger must occupy its slot before death",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Dead;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        None,
        "a dead ganger's occupant slot must be cleared (C4)",
    );
}

/// A DOWNED ganger frees its slot too — C4 covers Downed and Dead alike (only
/// non-Alive frees the cell).
#[test]
fn downed_ganger_clears_its_slot() {
    let mut app = headless_app();
    let at = key(20, 20, 0);

    let ganger = app
        .world_mut()
        .spawn((Position::new(at), LifeState::Alive))
        .id();
    app.update();

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Downed;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        None,
        "a downed ganger's occupant slot must be cleared (C4)",
    );
}

/// A `LifeState` change that stays [`LifeState::Alive`] does NOT free the slot —
/// only going OUT (Downed / Dead) clears it (C4).
#[test]
fn still_alive_change_keeps_slot() {
    let mut app = headless_app();
    let at = key(7, 7, 1);

    let ganger = app
        .world_mut()
        .spawn((Position::new(at), LifeState::Alive))
        .id();
    app.update();

    // Touch LifeState (mark it changed) but leave it Alive.
    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Alive;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "an Alive LifeState change must NOT free the slot (C4)",
    );
}

/// C9(c) — emitting a [`CoverDestroyed`] message and ticking once adds the cell
/// to the grid's destroyed-cover set.
///
/// Writes the message into the world buffer, ticks, and asserts the cell now
/// reads destroyed (and an unrelated cell does not). In place — no rebuild.
#[test]
fn cover_destroyed_message_marks_the_cell() {
    let mut app = headless_app();
    let smashed = key(30, 31, 3);
    let intact = key(0, 0, 0);

    app.world_mut().write_message(CoverDestroyed::new(smashed));
    app.update();

    assert_eq!(
        cover_destroyed(&app, smashed),
        Some(true),
        "a CoverDestroyed message must mark its cell destroyed (C5)",
    );
    assert_eq!(
        cover_destroyed(&app, intact),
        Some(false),
        "an unrelated cell must not be marked destroyed",
    );
}

/// Two consecutive moves keep the grid consistent — the second move clears the
/// FIRST destination (now the tracked previous slot), not the original start.
/// Proves the [`PrevSlot`] bookkeeping advances with each move.
#[test]
fn two_moves_track_the_previous_slot() {
    let mut app = headless_app();
    let a = key(1, 1, 0);
    let b = key(2, 2, 0);
    let c = key(3, 3, 0);

    let ganger = app
        .world_mut()
        .spawn((Position::new(a), LifeState::Alive))
        .id();
    app.update();

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(b);
    }
    app.update();

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(c);
    }
    app.update();

    assert_eq!(grid_occupant(&app, a), None, "the original start is empty");
    assert_eq!(
        grid_occupant(&app, b),
        None,
        "the first destination is empty"
    );
    assert_eq!(
        grid_occupant(&app, c),
        Some(ganger),
        "only the latest destination holds the ganger",
    );
}

/// The [`PrevSlot`] newtype round-trips the slot it records — the bookkeeping
/// the move system relies on.
#[test]
fn prev_slot_round_trips() {
    let slot = key(4, 5, 6);
    assert_eq!(PrevSlot::new(slot).slot(), slot);
    assert_eq!(PrevSlot::new(slot).upper(), None, "new() has no upper");
}

/// GTW-391: [`PrevSlot::with_upper`] round-trips both the lower and upper cells.
#[test]
fn prev_slot_with_upper_round_trips() {
    let lower = key(4, 5, 2);
    let upper = key(4, 5, 3);
    let ps = PrevSlot::with_upper(lower, upper);
    assert_eq!(ps.slot(), lower, "slot() returns the lower cell");
    assert_eq!(
        ps.upper(),
        Some(upper),
        "upper() returns the recorded upper cell"
    );
}

/// AC1 — [`SimSystems::Simulate`] is a public, hashable ordering set with the
/// required derives. Referencing the variant from the (in-crate, but
/// `pub`-reachable) test path and asserting equality/clone proves the variant is
/// public and that `Clone`/`Copy`/`PartialEq`/`Eq` are present; `cargo dbuild`
/// linking the binary confirms it is reachable downstream with no `unreachable_pub`.
#[test]
fn sim_systems_simulate_is_public_and_derives() {
    let set = SimSystems::Simulate;
    assert_eq!(
        set,
        SimSystems::Simulate,
        "the set compares equal to itself"
    );
    // `Copy` (a use after `set` was already read) and `Clone` both hold.
    assert_eq!(set, set.clone(), "the set clones to an equal value");
}

// ---------------------------------------------------------------------------
// GTW-391 — dual-cell stair occupancy presence
// ---------------------------------------------------------------------------

/// Mark `cell` as a stair tile in the app's [`OccupancyGrid`] resource.
fn mark_stair(app: &mut App, cell: CellLevel) {
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.mark_stair_cell(cell);
    }
}

/// GTW-391 Test 1: a Standing ganger on a stair tile registers BOTH the lower cell
/// (correct stance band) AND the upper cell (Low band) after the first tick.
#[test]
fn standing_stair_occupant_registers_upper_low_band() {
    let mut app = headless_app();
    let lower = key(10, 10, 1);
    let upper = key(10, 10, 2);

    mark_stair(&mut app, lower);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(lower),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(
        grid_occupant(&app, lower),
        Some(ganger),
        "lower cell must be occupied",
    );
    assert_eq!(
        grid_band(&app, lower),
        Some(HeightBand::High),
        "lower cell must carry the stance band (Standing → High)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper cell must also be occupied (dual-cell stair presence, GTW-391)",
    );
    assert_eq!(
        grid_band(&app, upper),
        Some(HeightBand::Low),
        "upper cell carries the Low band (body protrusion into the storey above, GTW-391)",
    );
}

/// GTW-391 C2: a Crouching ganger on a stair tile registers BOTH the lower cell
/// (Crouching's Mid silhouette band) AND the upper cell (Low band) after the first
/// tick — the guard is `stance_kind != StanceKind::Prone`, so any non-prone stance
/// gets the upper presence, not only Standing.
#[test]
fn crouching_stair_occupant_registers_upper_low_band() {
    let mut app = headless_app();
    let lower = key(10, 10, 1);
    let upper = key(10, 10, 2);

    mark_stair(&mut app, lower);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(lower),
            Stance::new(StanceKind::Crouching),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(
        grid_occupant(&app, lower),
        Some(ganger),
        "lower cell must be occupied",
    );
    assert_eq!(
        grid_band(&app, lower),
        Some(HeightBand::Mid),
        "lower cell must carry the stance band (Crouching → Mid)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper cell must also be occupied for a Crouching stair occupant (non-prone, GTW-391 C2)",
    );
    assert_eq!(
        grid_band(&app, upper),
        Some(HeightBand::Low),
        "upper cell carries the Low band regardless of the non-prone stance (GTW-391 C2)",
    );
}

/// GTW-391 Test 4: a ground shooter's lower-cell path is unchanged — no upper
/// presence on a non-stair cell.
#[test]
fn non_stair_occupant_lower_only() {
    let mut app = headless_app();
    let at = key(5, 5, 0);
    // NOT marked as a stair — the upper cell must remain empty.
    let upper = key(5, 5, 1);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(grid_occupant(&app, at), Some(ganger));
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "a non-stair occupant must not write an upper-cell presence",
    );
    assert_eq!(grid_band(&app, upper), None);
}

/// GTW-391 Test 5: a Prone ganger on a stair tile has NO upper-cell presence.
#[test]
fn prone_stair_occupant_has_no_upper_presence() {
    let mut app = headless_app();
    let lower = key(12, 12, 0);
    let upper = key(12, 12, 1);

    mark_stair(&mut app, lower);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(lower),
            Stance::new(StanceKind::Prone),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(grid_occupant(&app, lower), Some(ganger));
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "a prone stair occupant must have no upper-cell presence (prone = lower-only)",
    );
    assert_eq!(grid_band(&app, upper), None);
}

/// GTW-391 Test 6: moving OFF a stair clears the upper-cell presence — the
/// stale-registration #1 risk.
#[test]
fn move_off_stair_clears_upper_presence() {
    let mut app = headless_app();
    let stair = key(8, 8, 2);
    let upper = key(8, 8, 3);
    let dest = key(9, 9, 2); // non-stair destination

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    // Verify stair presence was established.
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence must be written on the stair",
    );

    // Move off the stair.
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        None,
        "old lower (stair) slot must be cleared",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "old upper slot must be cleared when moving off the stair (GTW-391)",
    );
    assert_eq!(grid_band(&app, upper), None, "old upper band must be gone");
    assert_eq!(
        grid_occupant(&app, dest),
        Some(ganger),
        "new destination must be occupied",
    );
    // dest is non-stair, so no upper presence there.
    assert_eq!(grid_occupant(&app, key(9, 9, 3)), None);
}

/// GTW-391 Test 7: going prone IN PLACE on a stair clears the upper presence —
/// the subtle same-cell-but-stance-changes case.
#[test]
fn go_prone_in_place_on_stair_clears_upper() {
    let mut app = headless_app();
    let stair = key(3, 3, 0);
    let upper = key(3, 3, 1);

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence must exist before going prone",
    );

    // Go prone in place.
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        Some(ganger),
        "lower slot stays occupied (still on the stair)",
    );
    assert_eq!(
        grid_band(&app, stair),
        Some(HeightBand::Low),
        "lower slot now carries Low band (Prone)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "upper slot must be cleared when going prone in place (GTW-391 Test 7)",
    );
    assert_eq!(grid_band(&app, upper), None);
}

/// GTW-391 Test 8: moving FROM one stair to ANOTHER stair relocates the upper
/// presence — old upper cleared, new upper written.
#[test]
fn stair_to_stair_move_relocates_upper() {
    let mut app = headless_app();
    let stair_a = key(5, 5, 1);
    let upper_a = key(5, 5, 2);
    let stair_b = key(15, 15, 1);
    let upper_b = key(15, 15, 2);

    mark_stair(&mut app, stair_a);
    mark_stair(&mut app, stair_b);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair_a),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(grid_occupant(&app, upper_a), Some(ganger));

    // Move to the other stair.
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(stair_b);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, upper_a),
        None,
        "old upper must be cleared on stair-to-stair move",
    );
    assert_eq!(grid_band(&app, upper_a), None);
    assert_eq!(
        grid_occupant(&app, upper_b),
        Some(ganger),
        "new upper must be written on the new stair (GTW-391)",
    );
    assert_eq!(grid_band(&app, upper_b), Some(HeightBand::Low));
}

/// GTW-391 Test 9: a ganger downed on a stair clears BOTH lower AND upper cells.
#[test]
fn dead_on_stair_clears_upper_presence() {
    let mut app = headless_app();
    let stair = key(7, 7, 3);
    let upper = key(7, 7, 4);

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence before death",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Downed;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        None,
        "lower slot must be cleared on death (C4)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "upper slot must also be cleared when downed on a stair (GTW-391 Test 9)",
    );
    assert_eq!(grid_band(&app, upper), None);
}

/// GTW-391 Test 10: a stair tile at the top storey (`MAX_LEVELS - 1`) writes no
/// upper presence — `upper_cell` returns `None` at the ceiling.
#[test]
fn top_storey_stair_occupant_lower_only() {
    use crate::metric::MAX_LEVELS;
    let mut app = headless_app();
    let top = key(2, 2, MAX_LEVELS - 1); // last valid storey

    mark_stair(&mut app, top);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(top),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(grid_occupant(&app, top), Some(ganger));
    // No upper cell exists above the top storey — register_stair_presence returns None.
    // The out-of-range key is graceful (no panic, no write).
    let non_existent_upper = key(2, 2, MAX_LEVELS);
    assert_eq!(
        grid_occupant(&app, non_existent_upper),
        None,
        "no upper-cell presence at the top storey (out-of-range, no panic)",
    );
}

/// GTW-391 Test 12: when the upper cell is already occupied by a DIFFERENT entity,
/// the stair ganger gets lower-only registration — the occupancy guard (Blocker 3).
#[test]
fn stair_occupant_with_occupied_upper_cell_is_lower_only() {
    let mut app = headless_app();
    let stair = key(20, 20, 1);
    let upper = key(20, 20, 2);

    mark_stair(&mut app, stair);

    // Ganger B occupies the upper cell as its OWN lower cell (it stands on level 2).
    let ganger_b = app
        .world_mut()
        .spawn((
            Position::new(upper),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger_b),
        "ganger B must occupy the upper cell before A registers",
    );

    // Now spawn ganger A on the stair — its upper cell is blocked by B.
    let ganger_a = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        Some(ganger_a),
        "ganger A occupies its lower (stair) cell",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger_b),
        "ganger B's slot must NOT be stomped by A (occupancy guard, GTW-391 Test 12)",
    );

    // Move A off the stair — B's slot must remain intact.
    let dest = key(21, 21, 1);
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger_a) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger_b),
        "B's slot survives A's teardown (teardown never stomps another entity's slot)",
    );
}

/// GTW-391 Test 13: `build_from_occupancy_input` with a stair-cell set registers the
/// upper presence AT PLACEMENT (frame 0 — Blocker 2 resolved).
#[test]
fn initial_placement_on_stair_registers_upper() {
    use bevy::{ecs::world::World, platform::collections::HashSet};

    use crate::occupancy::{OccupancyInput, OccupantPlacement};

    let mut world = World::new();
    let entity = world.spawn_empty().id();

    let lower = key(4, 4, 2);
    let upper = key(4, 4, 3);
    let stair_cells: HashSet<CellLevel> = std::iter::once(lower).collect();

    let input = OccupancyInput {
        terrain:   Vec::new(),
        occupants: vec![OccupantPlacement::new(lower, entity, HeightBand::High)],
    };

    // Build the grid — this must register the upper presence synchronously.
    let grid = OccupancyGrid::build_from_occupancy_input(&input, &stair_cells);

    // Assert WITHOUT any app.update() — the upper presence is frame-0 (Blocker 2).
    assert_eq!(
        grid.occupant(&lower),
        Some(entity),
        "lower cell occupied at build",
    );
    assert_eq!(
        grid.occupant_band(&lower),
        Some(HeightBand::High),
        "lower band set at build",
    );
    assert_eq!(
        grid.occupant(&upper),
        Some(entity),
        "upper cell occupied at build — frame-0, no tick needed (GTW-391 Test 13)",
    );
    assert_eq!(
        grid.occupant_band(&upper),
        Some(HeightBand::Low),
        "upper cell carries Low band at build",
    );
}

/// AC2/AC3 — after [`OccupancyMaintenancePlugin`] nests its chain under
/// [`SimSystems::Simulate`], the move-sync still fires through the set: a ganger
/// that moves clears its OLD slot and marks its NEW one within one `app.update()`,
/// proving set membership did not break execution (membership has no observable
/// beyond ordering + execution, so the behavioral assertion stands in for it).
#[test]
fn move_sync_fires_through_the_simulate_set() {
    let mut app = headless_app();
    let start = key(8, 8, 0);
    let dest = key(10, 12, 2);

    let ganger = app
        .world_mut()
        .spawn((Position::new(start), LifeState::Alive))
        .id();
    app.update();
    assert_eq!(
        grid_occupant(&app, start),
        Some(ganger),
        "initial placement marks the start slot through SimSystems::Simulate",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, start),
        None,
        "the OLD slot is cleared running inside SimSystems::Simulate",
    );
    assert_eq!(
        grid_occupant(&app, dest),
        Some(ganger),
        "the NEW slot is marked running inside SimSystems::Simulate",
    );
}
