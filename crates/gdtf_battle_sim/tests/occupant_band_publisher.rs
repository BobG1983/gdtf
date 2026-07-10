//! GTW-177 — the change-driven occupant-band publisher, proven on the REAL path.
//!
//! GTW-171 added `OccupancyGrid::occupant_band` / `set_occupant_band` (a
//! `(cell, level) -> HeightBand` side-map) as the band-free
//! [`march_vector`](gdtf_battle_sim::march::march_vector)'s way to read an occupant's
//! silhouette band for the clearance test. The production publisher that keeps that
//! side-map in lockstep with the occupant marker lives in `occupancy_sync`
//! (`sync_moved_gangers` publishes / clears the band on a move or re-pose;
//! `sync_dead_gangers` clears it on down / dead) and reuses the single
//! `silhouette_band` stance → band mapping (standing → HIGH, kneel → MID, prone → LOW).
//!
//! These tests exercise that REAL publisher — they NEVER call `set_occupant_band` in
//! the arrange (the hand-faking GTW-177 exists to remove). Each spawns a ganger into
//! the live [`OccupancyMaintenancePlugin`] chain, mutates the ganger's `Position` /
//! `Stance` / `LifeState` (which marks Bevy change-detection), runs `app.update()` so
//! the production maintenance chain republishes the side-map, then reads the band back
//! through the REAL [`march_vector`] — asserting by the round's verdict
//! ([`MarchKind`]), not by peeking a faked map.
//!
//! Pin-discrimination: the round band is chosen so its verdict FLIPS on the published
//! band. A standing target (HIGH) is impacted by a MID round; a prone target (LOW)
//! sails the same MID round over. So a test that would still pass if the publisher
//! never republished the new stance — i.e. left the stale standing-HIGH band at the
//! cell — is impossible: the stale band would keep impacting the MID round when the
//! contract requires it to clear.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY (`bevy-traps.md` #7 carve-out
//! (a)); no function here takes `&mut World` / `&World`. Zero pixels — the band is a
//! [`HeightBand`], the geometry is cubic-voxel `SimPos` / a unit `Vec3`.

use bevy::{
    app::App,
    math::Vec3,
    prelude::{Entity, MinimalPlugins},
};
use gdtf_battle_sim::{
    cover::{CoverLedger, HeightBand},
    march::{MarchDir, MarchKind, march_vector},
    metric::cell_center,
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        Cell, CellLevel, Level, LifeState, OccupancyGrid, Position, SimPos, Stance, StanceKind,
    },
    surface::SurfaceGrid,
    tuning::CombatTuning,
};

/// The cell the target ganger starts in.
fn start_cell() -> CellLevel {
    CellLevel::new(Cell::new(10, 5), Level::new(0))
}

/// The cell the target ganger MOVES to (C2(a) / C2(b)) — a clean second cell on the
/// same level so a flat round can be aimed through either independently.
fn moved_cell() -> CellLevel {
    CellLevel::new(Cell::new(10, 8), Level::new(0))
}

/// Build the real-path app: `MinimalPlugins` + the production
/// [`OccupancyMaintenancePlugin`] (whose `sync_moved_gangers` / `sync_dead_gangers`
/// ARE the publisher under test) plus the grids the maintenance chain mutates. NO
/// `set_occupant_band` call anywhere — the band is published only by the production
/// chain. The plugin's chain also runs `sync_destroyed_slab` / `sync_accrued_ground`
/// (which take `ResMut<SurfaceGrid>`) and `sync_destroyed_cover`, so the
/// [`SurfaceGrid`] / [`CoverLedger`] resources must be present for the chain's param
/// validation even though this ticket exercises only the occupant-band publish.
fn publisher_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(OccupancyMaintenancePlugin);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app
}

/// Spawn a target ganger at `at` holding `stance`, alive — a real `Position` +
/// `Stance` + `LifeState` so the production `sync_moved_gangers` publishes its
/// stance-derived silhouette band on the next `update()`.
fn spawn_target(app: &mut App, at: CellLevel, stance: StanceKind) -> Entity {
    app.world_mut()
        .spawn((Position::new(at), Stance::new(stance), LifeState::Alive))
        .id()
}

/// March a flat MID-band round EAST→through the column of `cell` (the round flies
/// along +x at the cell's mid-cell z), reporting the verdict. The muzzle sits a few
/// cells "west" (lower x) at the SAME y/z so the ray crosses `cell`; the shooter-cell
/// exception is parked far off-grid so it never suppresses the impact under test.
///
/// A MID round (z-fraction between the LOW→MID and MID→HIGH band edges) is the
/// discriminator: it IMPACTS an occupant banded MID or HIGH (equal-or-lower clears
/// the round's band), and SAILS OVER one banded LOW (strictly higher). Reads the
/// occupancy straight off the maintained grid — never a faked map.
fn march_mid_round_through(app: &App, cell: CellLevel) -> MarchKind {
    let tuning = CombatTuning::default();
    let Some(occupancy) = app.world().get_resource::<OccupancyGrid>() else {
        return MarchKind::Miss;
    };
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();

    // The MID z-fraction within the storey — strictly between the default band edges
    // (LOW→MID 0.33, MID→HIGH 0.67), derived from the tuning, never a literal.
    let mid_frac = f32::midpoint(
        *tuning.projectile_band_edges.low_mid,
        *tuning.projectile_band_edges.mid_high,
    );
    let center = cell_center(cell.cell(), Level::new(0));
    // Muzzle: same y, same MID z, a few cells west (lower x) so the flat +x ray
    // crosses `cell`'s column. Off-grid-x is avoided by keeping x >= 0.
    let muzzle = SimPos::new(center.x - 4.0, center.y, mid_frac);
    let dir = Vec3::new(1.0, 0.0, 0.0);
    // A far off-grid shooter cell — the muzzle's own cell is not the target, so the
    // shooter-cell exception never fires on the cell under test.
    let shooter_cell = CellLevel::new(Cell::new(59, 59), Level::new(7));

    march_vector(
        muzzle,
        MarchDir::new(dir),
        occupancy,
        &surface,
        &cover,
        &tuning,
        shooter_cell,
        |_| false,
    )
    .kind
}

/// C2(a) — after a MOVE **and** a STANCE CHANGE, the production publisher writes the
/// occupant's CURRENT band at the NEW cell, and the real march sees / acts on it.
///
/// Arrange: a STANDING target spawns at `start_cell` and one `update()` publishes its
/// HIGH band there. Act: move it to `moved_cell` AND re-pose it to PRONE, then
/// `update()` — the production `sync_moved_gangers` (`Changed<Position>` OR
/// `Changed<Stance>`) republishes the CURRENT (prone → LOW) band at the NEW cell.
/// Assert (the real read path):
/// - Via the real `march_vector`: a MID round through the NEW cell SAILS OVER the
///   now-LOW occupant (the round's MID band is strictly higher than LOW), so the
///   verdict is NOT `Ganger` at the new cell — proving the new band is NOT the stale
///   STANDING-HIGH one.
/// - Via the maintained grid: `occupant_band(new_cell)` reads exactly `Some(Low)` —
///   proving the occupant IS genuinely present there banded LOW (not simply absent),
///   so the MID-clears verdict is "sailed over a real LOW ganger", not "found nothing".
///
/// Pin-discrimination: had the publisher NOT republished the stance change, the new
/// cell would carry the stale STANDING-HIGH band (or, had it not republished the move,
/// no band at all). A stale HIGH band would IMPACT the MID round — flipping the first
/// assertion red; an absent/HIGH band would also flip the exact-`Some(Low)` read. The
/// pair (MID clears AND the band is exactly LOW at the new cell) is reachable ONLY if
/// the publisher wrote the fresh prone-LOW band at the new cell.
#[test]
fn move_plus_stance_change_publishes_current_band_at_new_cell() {
    let mut app = publisher_app();
    let target = spawn_target(&mut app, start_cell(), StanceKind::Standing);
    app.update(); // initial placement: HIGH band published at start_cell

    // Sanity on the initial publish (real code, no faking): a MID round impacts the
    // STANDING (HIGH) occupant at the start cell.
    assert_eq!(
        march_mid_round_through(&app, start_cell()),
        MarchKind::Ganger(target),
        "initial standing placement: a MID round impacts the HIGH occupant",
    );

    // Act: MOVE to the new cell AND re-pose to PRONE, in one frame — both mutations
    // mark change-detection, so `sync_moved_gangers` sees the entity and republishes.
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(target) {
        *pos = Position::new(moved_cell());
    }
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(target) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();

    // The CURRENT band (prone → LOW) is published at the NEW cell: a MID round sails
    // OVER it (MID strictly higher than LOW) — NOT a Ganger impact at the new cell.
    let mid_verdict = march_mid_round_through(&app, moved_cell());
    assert_ne!(
        mid_verdict,
        MarchKind::Ganger(target),
        "a MID round must sail OVER the now-PRONE (LOW) occupant at the new cell; a \
         stale STANDING-HIGH band would wrongly impact it (got {mid_verdict:?})",
    );

    // ...but the occupant IS there, banded LOW: the direct grid read confirms the
    // band the publisher wrote at the new cell is exactly LOW (prone), so the
    // MID-clears verdict above is "sailed over a real LOW ganger", not "found nothing".
    let published = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&moved_cell()));
    assert_eq!(
        published,
        Some(HeightBand::Low),
        "the publisher wrote the CURRENT prone (LOW) band at the new cell (got {published:?})",
    );
}

/// C2(b) — the VACATED cell (the one the target moved AWAY from) reads `None` after
/// the move: the production publisher clears the old cell's band in lockstep with its
/// occupant marker.
///
/// Arrange: a STANDING target spawns at `start_cell`; one `update()` publishes its
/// HIGH band there. Act: move it to `moved_cell`, `update()`. Assert via the real
/// `march_vector`: a MID round through the OLD `start_cell` no longer finds a ganger
/// (the band — and the occupant marker — were cleared), so the round passes through
/// (here a clean Miss off the grid edge, since nothing else is behind it).
///
/// Pin-discrimination: had the publisher NOT cleared the vacated cell, the old cell
/// would keep the STANDING-HIGH band and the MID round would still IMPACT the ganger
/// there — flipping this assertion red. A pass-through at the old cell is reachable
/// ONLY if the vacated cell's band was cleared to `None`.
#[test]
fn vacated_cell_reads_none_after_move() {
    let mut app = publisher_app();
    let target = spawn_target(&mut app, start_cell(), StanceKind::Standing);
    app.update(); // HIGH band published at start_cell

    // Before the move: the MID round impacts the HIGH occupant at start_cell.
    assert_eq!(
        march_mid_round_through(&app, start_cell()),
        MarchKind::Ganger(target),
        "pre-move: a MID round impacts the standing occupant at the start cell",
    );

    // Act: move to the new cell. The publisher clears the OLD cell (occupant + band).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(target) {
        *pos = Position::new(moved_cell());
    }
    app.update();

    // The vacated cell reads None: the MID round finds no ganger there and passes
    // through (a clean Miss, nothing behind it).
    let verdict = march_mid_round_through(&app, start_cell());
    assert_ne!(
        verdict,
        MarchKind::Ganger(target),
        "the vacated cell must read None — a MID round through the OLD cell must NOT \
         impact the ganger that left it (got {verdict:?})",
    );
    // Direct grid read: the band side-map has no entry at the vacated cell.
    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&start_cell()));
    assert_eq!(
        band, None,
        "the publisher cleared the vacated cell's band in lockstep with its occupant",
    );
}

/// C2(c) — on DOWN / DEAD the occupied cell reads `None`: the production
/// `sync_dead_gangers` clears the band (and the occupant marker) when the slot frees.
///
/// Arrange: a STANDING target spawns at `start_cell`; one `update()` publishes its
/// HIGH band there. Act: transition its `LifeState` to `Dead` (`Changed<LifeState>`),
/// `update()`. Assert via the real `march_vector`: a MID round through `start_cell` no
/// longer finds a ganger — the freed slot's band is cleared, so the round passes
/// through.
///
/// GTW-459: only a `Dead` ganger frees its slot (occupant + band) in
/// `sync_dead_gangers` — a `Downed` ganger HOLDS its cell (a body still on the field,
/// blocking movement and OCCLUDING fire; `docs/combat/resolution.md` §9: a live
/// occupant, INCLUDING a downed one, still stops the round). This test therefore
/// exercises the `Dead` arm — the corpse path that genuinely clears the band. The
/// `Downed`-retains-its-band invariant is pinned in the `occupancy_sync` suite
/// (`downed_ganger_retains_its_slot`) and the GTW-317 pass-through-dead suite.
///
/// Pin-discrimination: had the publisher NOT cleared the cell on death, the band would
/// remain STANDING-HIGH and the MID round would still IMPACT — flipping this red. A
/// pass-through after death is reachable ONLY if the freed cell's band was cleared.
#[test]
fn dead_occupant_cell_reads_none() {
    let mut app = publisher_app();
    let target = spawn_target(&mut app, start_cell(), StanceKind::Standing);
    app.update(); // HIGH band published at start_cell

    // Before death: the MID round impacts the HIGH occupant.
    assert_eq!(
        march_mid_round_through(&app, start_cell()),
        MarchKind::Ganger(target),
        "pre-death: a MID round impacts the live standing occupant",
    );

    // Act: the ganger DIES. `sync_dead_gangers` frees its slot (occupant + band).
    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(target) {
        *life = LifeState::Dead;
    }
    app.update();

    // The freed cell reads None: the MID round finds no ganger and passes through.
    let verdict = march_mid_round_through(&app, start_cell());
    assert_ne!(
        verdict,
        MarchKind::Ganger(target),
        "on DEATH the occupied cell must read None — a MID round must NOT impact the \
         dead ganger's vacated cell (got {verdict:?})",
    );
    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&start_cell()));
    assert_eq!(
        band, None,
        "the publisher cleared the freed cell's band when the ganger died (GTW-459)",
    );
}
