//! End-to-end emit-step tests (GTW-431 C2/C3; GTW-492 v2 model): the full space-packing
//! pipeline (assemble -> fill -> emit) is DETERMINISTIC under a fixed [`ProcgenRng`] seed
//! (same seed -> identical emitted terrain; different seeds -> different terrain, so the
//! determinism pin is not vacuous), and the emitted [`Situation`] is a VALID assembled level
//! — connected (seam-reachable) and in-bounds. The REAL pipeline is driven with an injected
//! seeded RNG over the UUID-keyed v2 prefab model ([`PrefabRegistry2`] of [`Prefab2`]) +
//! the [`UuidThemeRegistry`] (for the theme's default floor) + the [`TerrainDefRegistry`]
//! (classifying each placed piece); the assertions are on the EMITTED output, never on a
//! reimplementation.

use bevy::asset::uuid::Uuid;

use crate::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab2, PrefabName, PrefabRegistry2,
        PrefabSpecV2, SpawnRole, TerrainPlacementEntry, ThemeDisplayName, ThemeUuid, UuidThemeDef,
        UuidThemeRegistry,
    },
    metric::{Cell, CellLevel, Level},
    procgen::{
        DeadRectScatterCount, LargePrefabAreaThreshold, MinDensityFloor, ProcgenTuning, RegionRect,
        generate_level,
    },
    rng::{BattleSeed, ProcgenRng},
    situation::Situation,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

/// A `(cell, level)` on level 0 (a tiny helper).
fn at(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A footprint / board `GridSize` (clamped; `None` returns early — never panics).
fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

/// The canonical test theme key — a fixed `from_u128` [`ThemeUuid`] every prefab + the
/// `UuidThemeRegistry` author, so the generated level's theme resolves to its default floor.
fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0149_2431_0000_0001))
}

/// The test WALL terrain def UUID a prefab places (the canonical sim test-support `WALL`
/// piece, a `Wall` sim-kind — so the emit classifies it into the `walls` list).
fn wall_piece() -> TerrainUuid {
    crate::test_support::test_pieces::WALL
}

/// The test FLOOR terrain def UUID the theme nominates as its default floor (the
/// canonical sim test-support `FLOOR` piece).
fn floor_piece() -> TerrainUuid {
    crate::test_support::test_pieces::FLOOR
}

/// A v2 prefab of `role` at footprint `fp` that AUTHORS a wall placement at the
/// footprint-local cell `(1, 1)` — so the emit has real terrain to translate (and so
/// different anchors, i.e. different placed origins, produce different translated cells: the
/// determinism pin is then discriminating).
fn prefab(theme: ThemeUuid, fp: GridSize, role: SpawnRole, stem: &str) -> Prefab2 {
    Prefab2::new(
        PrefabName::new(stem.to_owned()),
        PrefabSpecV2::new(
            theme,
            fp,
            role,
            vec![TerrainPlacementEntry::new(wall_piece(), at(1, 1))],
        ),
    )
}

/// A registry with a player + enemy deployment prefab and the given `Fill` prefabs (each
/// `(stem, w, h)`), every prefab authoring a wall so the emit produces real terrain.
fn registry_with_fill(
    theme: ThemeUuid,
    player_fp: GridSize,
    enemy_fp: GridSize,
    fills: &[(&str, u8, u8)],
) -> Option<PrefabRegistry2> {
    let mut r = PrefabRegistry2::default();
    r.insert(prefab(theme, player_fp, SpawnRole::Player, "player_pad"));
    r.insert(prefab(theme, enemy_fp, SpawnRole::Enemy, "enemy_pad"));
    for (stem, w, h) in fills {
        let fp = size(*w, *h)?;
        r.insert(prefab(theme, fp, SpawnRole::Fill, stem));
    }
    Some(r)
}

/// A theme registry naming `theme()` with the test FLOOR piece as its default floor — so the
/// emitted level's `default_floor` resolves to a real (non-nil) terrain UUID (GTW-492).
fn theme_registry(theme: ThemeUuid) -> UuidThemeRegistry {
    UuidThemeRegistry::new([(
        theme,
        UuidThemeDef {
            key:           theme,
            display_name:  ThemeDisplayName::new("Test Theme".to_owned()),
            default_floor: floor_piece(),
            terrain:       vec![wall_piece(), floor_piece()],
        },
    )])
}

/// The canonical test terrain-def registry (WALL / SLAB / COVER / FLOOR) the emit classifies
/// placed pieces against (`wall_piece()` resolves to a `Wall` sim-kind → the walls list).
fn terrain_defs() -> TerrainDefRegistry {
    crate::test_support::test_terrain_registry()
}

/// A tuning with explicit knob values (the unit tests drive the knobs directly).
fn tuning(density: f32, large_area: u32, scatter_k: u8) -> ProcgenTuning {
    ProcgenTuning {
        min_density_floor:           MinDensityFloor::new(density),
        large_prefab_area_threshold: LargePrefabAreaThreshold::new(large_area),
        dead_rect_scatter_count_k:   DeadRectScatterCount::new(scatter_k),
    }
}

/// Whether two emitted situations have STRUCTURALLY EQUAL terrain entries — the C2
/// equality measure (the terrain leaves all derive `Eq`; `Situation` itself does not, so we
/// compare the terrain-entry fields directly).
fn terrain_eq(a: &Situation, b: &Situation) -> bool {
    a.theme == b.theme
        && a.grid_size == b.grid_size
        && a.default_floor == b.default_floor
        && a.walls == b.walls
        && a.scatter == b.scatter
        && a.slabs == b.slabs
        && a.floors == b.floors
        && a.vertical_links == b.vertical_links
}

/// C2 (determinism): the FULL pipeline (assemble -> fill -> emit) is deterministic under a
/// fixed seed — two runs from the SAME seed emit terrain-EQUAL situations, AND two runs from
/// DIFFERENT seeds emit terrain-DIFFERENT situations (so the determinism pin is not vacuous:
/// it would pass trivially if every seed produced the same empty/identical level).
///
/// Discriminating: a non-deterministic pipeline (unsorted map iteration, non-seeded draw)
/// would make the same-seed runs differ; a pipeline that ignored the seed would make the
/// different-seed runs identical. The test fails either way.
#[test]
fn pipeline_emit_is_deterministic_under_a_seed() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(prefabs) = registry_with_fill(
        theme,
        player_fp,
        enemy_fp,
        &[("hall", 8, 8), ("nook", 4, 4), ("room", 6, 6)],
    ) else {
        return;
    };
    let themes = theme_registry(theme);
    let terrain_defs = terrain_defs();
    let knobs = tuning(0.9, 49, 2);

    let run = |seed: BattleSeed| -> Option<Situation> {
        let mut rng = ProcgenRng::from_root(seed);
        generate_level(
            &prefabs,
            &themes,
            &terrain_defs,
            theme,
            board,
            &mut rng,
            &knobs,
        )
        .ok()
    };

    let seed = BattleSeed::new(0x5EED_4311);
    let (Some(a), Some(b)) = (run(seed), run(seed)) else {
        return;
    };
    assert!(
        terrain_eq(&a, &b),
        "the same seed must emit an IDENTICAL assembled level (C2 determinism)",
    );

    // A different seed must emit DIFFERENT terrain (the player anchor — the one RNG draw —
    // differs, so the placed origins, and thus the translated wall cells, differ). This
    // makes the determinism pin discriminating, not vacuously equal.
    let Some(c) = run(BattleSeed::new(0xD1FF_4311)) else {
        return;
    };
    // Try several alternative seeds — at least one must diverge from `a` (the anchor draw is
    // a small enum, so a given pair could collide; the pin holds if ANY differs).
    let differs = !terrain_eq(&a, &c)
        || [1u64, 2, 3, 5, 8, 13, 21, 34]
            .into_iter()
            .filter_map(|s| run(BattleSeed::new(s)))
            .any(|other| !terrain_eq(&a, &other));
    assert!(
        differs,
        "different seeds must be able to emit DIFFERENT assembled levels (the determinism \
         pin is discriminating, not vacuously equal)",
    );
}

/// C3 (validity): the emitted level is a VALID assembled level — every authored terrain
/// cell is IN-BOUNDS (within the board footprint, non-negative), it carries the
/// translated prefab walls (poured into the `walls` list by the def's `Wall` sim-kind), and
/// it is CONNECTED (the player + enemy + fill regions are all seam-reachable from the player
/// region; a disconnected level would have returned a fail-closed `Disconnected` error
/// instead of `Ok`).
///
/// Discriminating: an emit that forgot to translate footprint-local cells onto the board
/// (or shipped a disconnected level) would fail the in-bounds / connectivity checks; a
/// no-op emit would carry no walls. An emit that iterated four split lists (the legacy
/// schema) would carry NO walls (a v2 prefab has none), so a non-empty `walls` list also
/// pins that the single placements list was poured (C1).
#[test]
fn emitted_level_is_in_bounds_and_connected() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(prefabs) = registry_with_fill(
        theme,
        player_fp,
        enemy_fp,
        &[("hall", 8, 8), ("nook", 4, 4)],
    ) else {
        return;
    };
    let themes = theme_registry(theme);
    let terrain_defs = terrain_defs();
    let knobs = tuning(0.8, 49, 2);

    let mut rng = ProcgenRng::from_root(BattleSeed::new(0xB0_1234));
    let result = generate_level(
        &prefabs,
        &themes,
        &terrain_defs,
        theme,
        board,
        &mut rng,
        &knobs,
    );
    assert!(
        result.is_ok(),
        "the emit must succeed (a connected level): {:?}",
        result.as_ref().err(),
    );
    let Ok(situation) = result else {
        return;
    };

    // GTW-492: the theme is the UUID-keyed key directly (no shim), and the default_floor
    // resolved from the theme registry (the test FLOOR piece).
    assert_eq!(
        situation.theme, theme,
        "the emitted level's theme must be the requested ThemeUuid (no shim)",
    );

    // The emit reached the connectivity assertion and returned Ok, so the level is connected
    // by that fail-closed check. The emitted level carries the translated prefab walls (the
    // single v2 placements list poured into `walls` by the Wall def classification).
    assert!(
        !situation.walls.is_empty(),
        "the emitted level must carry the translated prefab walls (C1/C3 — the v2 placements \
         list was poured into the situation, classified into the walls list)",
    );

    // Every authored terrain cell must be in-bounds: 0 <= x < board_w, 0 <= y < board_h.
    let board_rect = RegionRect::board(board);
    let board_w = board_rect.footprint().width();
    let board_h = board_rect.footprint().height();
    let in_bounds = |c: CellLevel| c.x >= 0 && c.x < board_w && c.y >= 0 && c.y < board_h;
    for w in &situation.walls {
        assert!(
            in_bounds(w.at),
            "every emitted wall cell must be in-bounds: {:?} on a {board_w}x{board_h} board",
            w.at,
        );
    }
    for s in &situation.scatter {
        assert!(
            in_bounds(s.at),
            "every emitted scatter cell must be in-bounds: {:?}",
            s.at
        );
    }
    for f in &situation.floors {
        assert!(
            in_bounds(f.at),
            "every emitted floored dead-space cell must be in-bounds: {:?}",
            f.at
        );
    }

    // The dead space was floored (C3): with a partial density floor there is leftover free
    // space, so the emit must have produced explicit `default_floor` overrides for it.
    assert!(
        !situation.floors.is_empty(),
        "the leftover dead space must be FLOORED with explicit default_floor entries (C3)",
    );
    assert!(
        !situation.default_floor.is_nil(),
        "the emitted level must carry a default_floor (the theme's nominated ground terrain)",
    );
    assert_eq!(
        situation.default_floor,
        floor_piece(),
        "the default_floor must resolve from the theme registry's nominated terrain (GTW-492)",
    );
}
