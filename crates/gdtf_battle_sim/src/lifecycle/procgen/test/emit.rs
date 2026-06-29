//! End-to-end emit-step tests (GTW-431 C2/C3): the full space-packing pipeline
//! (assemble -> fill -> emit) is DETERMINISTIC under a fixed [`ProcgenRng`] seed (same seed
//! -> identical emitted terrain; different seeds -> different terrain, so the determinism
//! pin is not vacuous), and the emitted [`Situation`] is a VALID assembled level — connected
//! (seam-reachable) and in-bounds. The REAL pipeline is driven with an injected seeded RNG;
//! the assertions are on the EMITTED output, never on a reimplementation.

use crate::{
    level::{
        EdgeOpening, GridHeight, GridLevels, GridSize, GridWidth, LevelTheme, Prefab, PrefabName,
        PrefabPiece, PrefabRegistry, PrefabSpec, SpawnRole,
    },
    metric::{Cell, CellLevel, Level},
    procgen::{
        DeadRectScatterCount, LargePrefabAreaThreshold, MinDensityFloor, ProcgenTuning, RegionRect,
        generate_level,
    },
    rng::{BattleSeed, ProcgenRng},
    situation::Situation,
    terrain::piece::TerrainName,
};

/// A `(cell, level)` on level 0 (a tiny helper).
fn at(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A footprint / board `GridSize` (clamped; `None` returns early — never panics).
fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

/// One validated prefab of `role` at footprint `fp` that AUTHORS a wall cell + the level
/// `default_floor` — so the emit has real terrain to translate (and so different anchors,
/// i.e. different placed origins, produce different translated cells: the determinism pin
/// is then discriminating). `None` if the size is invalid or the prefab fails validation.
fn prefab(theme: LevelTheme, fp: GridSize, role: SpawnRole, stem: &str) -> Option<Prefab> {
    let spec = PrefabSpec {
        theme,
        size: fp,
        spawn_role: role,
        default_floor: TerrainName::new("deck_floor".to_owned()),
        // A wall at the footprint-local cell (1, 1) — translated onto the board by the
        // placed region origin during emit (GTW-491: the legacy prefab fragment stays
        // `TerrainName`-keyed via `PrefabPiece`; emit bridges it to the UUID-keyed situation).
        walls: vec![PrefabPiece::new(
            at(1, 1),
            TerrainName::new("bulkhead".to_owned()),
        )],
        edge_openings: vec![EdgeOpening::new(at(0, 0))],
        ..PrefabSpec::default()
    };
    Prefab::new(PrefabName::new(stem.to_owned()), spec).ok()
}

/// A registry with a player + enemy deployment prefab and the given `Fill` prefabs (each
/// `(stem, w, h)`), every prefab authoring a wall so the emit produces real terrain.
fn registry_with_fill(
    theme: LevelTheme,
    player_fp: GridSize,
    enemy_fp: GridSize,
    fills: &[(&str, u8, u8)],
) -> Option<PrefabRegistry> {
    let mut r = PrefabRegistry::default();
    r.insert(prefab(theme, player_fp, SpawnRole::Player, "player_pad")?);
    r.insert(prefab(theme, enemy_fp, SpawnRole::Enemy, "enemy_pad")?);
    for (stem, w, h) in fills {
        let fp = size(*w, *h)?;
        r.insert(prefab(theme, fp, SpawnRole::Fill, stem)?);
    }
    Some(r)
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
    let theme = LevelTheme::IndustrialHive;
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(registry) = registry_with_fill(
        theme,
        player_fp,
        enemy_fp,
        &[("hall", 8, 8), ("nook", 4, 4), ("room", 6, 6)],
    ) else {
        return;
    };
    let knobs = tuning(0.9, 49, 2);

    let run = |seed: BattleSeed| -> Option<Situation> {
        let mut rng = ProcgenRng::from_root(seed);
        generate_level(&registry, theme, board, &mut rng, &knobs).ok()
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
/// translated prefab walls, and it is CONNECTED (the player + enemy + fill regions are all
/// seam-reachable from the player region; a disconnected level would have returned a
/// fail-closed `Disconnected` error instead of `Ok`).
///
/// Discriminating: an emit that forgot to translate footprint-local cells onto the board
/// (or shipped a disconnected level) would fail the in-bounds / connectivity checks; a
/// no-op emit would carry no walls.
#[test]
fn emitted_level_is_in_bounds_and_connected() {
    let theme = LevelTheme::Underhive;
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(registry) = registry_with_fill(
        theme,
        player_fp,
        enemy_fp,
        &[("hall", 8, 8), ("nook", 4, 4)],
    ) else {
        return;
    };
    let knobs = tuning(0.8, 49, 2);

    let mut rng = ProcgenRng::from_root(BattleSeed::new(0xB0_1234));
    let result = generate_level(&registry, theme, board, &mut rng, &knobs);
    assert!(
        result.is_ok(),
        "the emit must succeed (a connected level): {:?}",
        result.as_ref().err(),
    );
    let Ok(situation) = result else {
        return;
    };

    // The emit reached the connectivity assertion and returned Ok, so the level is connected
    // by that fail-closed check. The emitted level carries the translated prefab walls.
    assert!(
        !situation.walls.is_empty(),
        "the emitted level must carry the translated prefab walls (C3 — the prefab geometry \
         was poured into the situation)",
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
        "the emitted level must carry a default_floor (the seam-lattice floor)",
    );
}
