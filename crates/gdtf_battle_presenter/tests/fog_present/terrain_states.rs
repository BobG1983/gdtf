//! Three-state terrain treatment: visible / explored / unseen + the dense lit
//! floor around an observer (GTW-347/348).

use bevy::{app::App, prelude::Visibility};
use gdtf_battle_sim::{
    Cell, CellLevel, CombatTuning, CoverLedger, Direction, Facing, Faction, FovObserver, Level,
    LifeState, OccupancyGrid, Position, StairEyeOffset, Stance, StanceKind, SurfaceGrid,
    test_support::SituationBuilder, union_fov,
};

use super::harness::*;

/// AC (GTW-348) — terrain three-state: a squad-VISIBLE cell renders at full colour
/// (`TerrainFogMaterial.saturation == 1.0`); an EXPLORED-but-not-visible cell renders
/// full-brightness GREYSCALE (`saturation == 0.0` — colour-loss, not brightness-loss, as
/// the memory cue); an UNSEEN cell does not render its terrain (`Visibility::Hidden`).
///
/// Pin-discriminating: it asserts EXPLORED maps to `0.0` (greyscale) and VISIBLE to `1.0`
/// (colour); if EXPLORED were mapped to the wrong saturation (e.g. still dimmed, or left at
/// `1.0`), the EXPLORED assert fails.
#[test]
fn terrain_renders_visible_explored_unseen() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let visible_cell = CellLevel::new(Cell::new(5, 5), l0);
    let explored_cell = CellLevel::new(Cell::new(6, 6), l0);
    let unseen_cell = CellLevel::new(Cell::new(7, 7), l0);

    // One player ganger so setup spawns a valid battle (its cell is fogged-VISIBLE).
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(visible_cell, 0, Direction::East))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, unseen_cell),
        "the terrain field must have drawn (every in-range cell is at least floor)",
    );

    // Author the fog: visible_cell VISIBLE, explored_cell EXPLORED-only, unseen_cell neither.
    set_fog(&mut app, &[visible_cell], &[explored_cell]);
    app.update();

    // VISIBLE -> full colour (saturation 1.0), shown.
    let visible_terrain = terrain_at(&mut app, visible_cell);
    assert!(
        visible_terrain.is_some(),
        "the VISIBLE cell must have a terrain tile + material"
    );
    let Some((vis_saturation, vis_flag)) = visible_terrain else {
        return;
    };
    assert!(
        (vis_saturation - 1.0).abs() < f32::EPSILON,
        "a squad-VISIBLE cell renders at full colour (saturation 1.0); got {vis_saturation}",
    );
    assert_eq!(vis_flag, Visibility::Inherited, "VISIBLE terrain is shown");

    // EXPLORED -> full-brightness greyscale (saturation 0.0), shown.
    let explored_terrain = terrain_at(&mut app, explored_cell);
    assert!(
        explored_terrain.is_some(),
        "the EXPLORED cell must have a terrain tile + material"
    );
    let Some((exp_saturation, exp_flag)) = explored_terrain else {
        return;
    };
    assert!(
        exp_saturation.abs() < f32::EPSILON,
        "an EXPLORED cell renders full-brightness GREYSCALE (saturation 0.0 — colour-loss as \
         the memory cue, not brightness-loss); got {exp_saturation}",
    );
    assert!(
        exp_saturation < vis_saturation,
        "EXPLORED is desaturated relative to VISIBLE (the memory cue is colour-loss)",
    );
    assert_eq!(
        exp_flag,
        Visibility::Inherited,
        "EXPLORED terrain is shown (greyscale, full brightness)",
    );

    // UNSEEN -> hidden.
    let unseen_terrain = terrain_at(&mut app, unseen_cell);
    assert!(
        unseen_terrain.is_some(),
        "the UNSEEN cell must have a terrain tile + material"
    );
    let Some((_unseen_saturation, unseen_flag)) = unseen_terrain else {
        return;
    };
    assert_eq!(
        unseen_flag,
        Visibility::Hidden,
        "a never-seen cell does not render its terrain",
    );
}

/// Compute the squad VISIBLE set the REAL way (GTW-347): run [`union_fov`]'s dense
/// Chebyshev disc scan from a single conscious player observer at `at`, over the world's
/// live sim grids (`OccupancyGrid` / `SurfaceGrid` / `CoverLedger`). Returns the visible
/// cells as a `Vec` (the test then authors them into the fog and asserts the presenter
/// renders them) — the same dense floor the presenter draws.
///
/// `is_dead` is the no-corpse predicate (no occupant is a corpse on the flat fixtures).
fn dense_visible_from_observer(app: &App, at: CellLevel) -> Vec<CellLevel> {
    let occupancy = app.world().resource::<OccupancyGrid>().clone();
    let surface = app.world().resource::<SurfaceGrid>().clone();
    let cover = app.world().resource::<CoverLedger>().clone();
    let tuning = app.world().resource::<CombatTuning>().clone();
    // A standing, East-facing conscious observer at `at` — the borrow-view union_fov needs.
    let position = Position::new(at);
    let stance = Stance::new(StanceKind::Standing);
    let facing = Facing::new(Direction::East);
    let observers = [FovObserver {
        position:         &position,
        stance:           &stance,
        facing:           &facing,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];
    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, |_| false);
    visible.into_iter().collect()
}

/// AC (GTW-347 clause 6 — the regression-fix render proof): with the **dense** VISIBLE set
/// `union_fov` produces around a player observer, the presenter renders the open-floor
/// `TerrainSprite`s on visible cells at full colour / `Visibility::Inherited` (NOT the
/// all-black bug), while a truly-unseen (out-of-range) floor cell stays `Hidden`.
///
/// This wires the REAL sim FOV (the dense disc scan over the live grids) into the REAL
/// presenter fog writer — proving the fix end to end: pre-GTW-347 the dense set was the
/// sparse authored/occupied subset, so an all-Open floor revealed nothing and the whole
/// terrain layer rendered Hidden (the all-black floor).
#[test]
fn dense_floor_set_renders_lit_floor_around_observer() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    // The player observer's cell, and an in-range open-floor neighbour the dense scan
    // reveals (both are open floor — no terrain authored at them).
    let observer_cell = CellLevel::new(Cell::new(10, 10), l0);
    let near_floor = CellLevel::new(Cell::new(11, 10), l0);
    // A floor cell far beyond the default view_range (14 Chebyshev) — never revealed.
    let far_floor = CellLevel::new(Cell::new(40, 40), l0);

    // One player ganger at the observer cell so setup spawns a valid battle; the rest of
    // the grid is open floor (the presenter floors every in-range Open cell).
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(observer_cell, 0, Direction::East))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, far_floor),
        "the open-floor terrain field must have drawn (every in-range cell is at least floor)",
    );

    // Build the fog the REAL way: union_fov's dense disc scan from the player observer.
    let visible = dense_visible_from_observer(&app, observer_cell);
    assert!(
        visible.contains(&observer_cell) && visible.contains(&near_floor),
        "the dense scan reveals the observer's open-floor cell + an in-range floor neighbour",
    );
    assert!(
        !visible.contains(&far_floor),
        "a floor cell beyond view_range is NOT in the dense VISIBLE set",
    );
    set_fog(&mut app, &visible, &[]);
    app.update();

    // The presenter renders the VISIBLE open-floor cells LIT (full colour, shown) — the
    // lit disc around the squad, NOT the all-black bug.
    let observer_terrain = terrain_at(&mut app, observer_cell);
    let near_terrain = terrain_at(&mut app, near_floor);
    assert!(
        observer_terrain.is_some() && near_terrain.is_some(),
        "the observer + neighbour floor cells must have terrain sprites",
    );
    let (Some((obs_saturation, obs_flag)), Some((near_saturation, near_flag))) =
        (observer_terrain, near_terrain)
    else {
        return;
    };
    assert!(
        (obs_saturation - 1.0).abs() < f32::EPSILON,
        "the observer's open-floor cell renders at full colour (saturation 1.0 — lit, not black); \
         got {obs_saturation}",
    );
    assert_eq!(
        obs_flag,
        Visibility::Inherited,
        "the observer's open-floor cell is shown",
    );
    assert!(
        (near_saturation - 1.0).abs() < f32::EPSILON,
        "an in-range open-floor cell renders LIT at full colour (the dense fog reveals the \
         rendered floor); got {near_saturation}",
    );
    assert_eq!(
        near_flag,
        Visibility::Inherited,
        "an in-range open-floor cell is shown",
    );

    // The out-of-range floor cell stays Hidden (the fog horizon — dark beyond view_range).
    let far_terrain = terrain_at(&mut app, far_floor);
    assert!(
        far_terrain.is_some(),
        "the far open-floor cell must have a terrain tile (every cell is at least floor)",
    );
    let Some((_far_saturation, far_flag)) = far_terrain else {
        return;
    };
    assert_eq!(
        far_flag,
        Visibility::Hidden,
        "a floor cell beyond view_range stays Hidden — the fog horizon, not an all-black map",
    );
}
