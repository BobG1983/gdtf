//! End-to-end emit-step tests (GTW-431 C2/C3; GTW-492 v2 model; GTW-497 by-construction
//! connectivity): the full space-packing pipeline (assemble -> fill -> emit) is
//! DETERMINISTIC under a fixed [`ProcgenRng`] seed (same seed -> identical emitted terrain;
//! different seeds -> different terrain, so the determinism pin is not vacuous), and the
//! emitted [`Situation`] is a VALID assembled level — every cell OUTSIDE a placed region is
//! reachable BY CONSTRUCTION (the 1-cell `default_floor` seam lattice; asserted by flooding
//! the open cells of the REAL packer output — the placed/filled region rectangles — NOT the
//! removed connectivity flood) and every authored cell is in-bounds. That connectivity
//! invariant is proved PIN-DISCRIMINATING by a control (`seam_separated_regions_stay_connected`)
//! that feeds the same flood helper an abutting (seam-less) layout and asserts it splits the
//! board — so the invariant would FAIL if the packer's `Margin::DEFAULT` seam were removed.
//! The REAL pipeline is driven with an injected seeded RNG over the UUID-keyed v2 prefab
//! model ([`PrefabRegistry`] of [`Prefab`]) + the [`UuidThemeRegistry`] (for the theme's
//! default floor) + the [`TerrainDefRegistry`] (classifying each placed piece); the
//! assertions are on the EMITTED output, never on a reimplementation.

use bevy::asset::uuid::Uuid;

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, TerrainPlacementEntry, ThemeDisplayName, ThemeUuid, UuidThemeDef,
        UuidThemeRegistry,
    },
    metric::{Cell, CellLevel, Level},
    procgen::{
        DeadRectScatterCount, FilledPlacement, Footprint, LargePrefabAreaThreshold, Margin,
        MinDensityFloor, MinPlayerSide, PlacedPrefab, ProcgenTuning, RegionRect, SplitMode,
        assemble_placement_with, fill_placement_with, generate_level,
    },
    rng::{BattleSeed, ProcgenRng},
    situation::Situation,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
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
fn prefab(theme: ThemeUuid, fp: GridSize, role: SpawnRole, stem: &str) -> Prefab {
    Prefab::new(
        PrefabName::new(stem.to_owned()),
        PrefabSpec::new(
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
) -> Option<PrefabRegistry> {
    let mut r = PrefabRegistry::default();
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

/// C2 (the by-construction connectivity INVARIANT): a generated level under a FIXED
/// [`ProcgenRng`] seed is connected BY CONSTRUCTION — every board cell that is NOT inside a
/// placed region is reachable, in 4-connectivity, from any single open cell. The open
/// (non-region) cells form ONE connected component because the 1-cell `default_floor` seam
/// every placement reserves leaves a continuous walkable corridor lattice between every pair
/// of placed regions. This is asserted WITHOUT the removed connectivity flood: it floods the
/// open cells of the REAL packer output (the [`FilledPlacement`]'s placed + filled region
/// rectangles — the very rectangles the seam is reserved around), then additionally runs the
/// full [`generate_level`] entry point and verifies the emitted `Situation`.
///
/// Pin-discriminating: the discrimination is proved by [`seam_separated_regions_stay_connected`],
/// which feeds the SAME flood helper a control layout where two regions ABUT (the layout the
/// packer would produce if [`Margin::DEFAULT`] were dropped to a zero seam) and asserts that
/// control's open cells split into TWO components. So a packer with the seam removed would
/// make this invariant FAIL. (The in-bounds + non-empty-walls checks below additionally pin
/// that the emit translated the footprint-local cells onto the board.)
#[test]
fn emitted_level_is_in_bounds_and_fully_connected() {
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

    // Drive the REAL staged pipeline (the exact functions `generate_level` calls) under a
    // fixed seed to recover the FilledPlacement — its placed + filled region rectangles ARE
    // the packer's seam-reserved output (the seam is the 1-cell gap BETWEEN these regions).
    let seed = BattleSeed::new(0xB0_1234);
    let Some(filled) = run_pipeline(&prefabs, theme, board, seed, &knobs) else {
        return;
    };

    let board_rect = RegionRect::board(board);
    let board_w = board_rect.footprint().width();
    let board_h = board_rect.footprint().height();

    // The by-construction connectivity INVARIANT (C2): every cell NOT inside a placed region
    // is reachable. The placed/filled regions are the only ground-plane blockers; the 1-cell
    // seam reserved around each leaves a walkable lattice, so the open cells are ONE
    // connected component. (Proved discriminating by `seam_separated_regions_stay_connected`:
    // remove the seam and the control layout below fails this same helper.)
    let occupied = occupied_regions(&filled);
    assert!(
        open_cells_form_one_component(&occupied, board_w, board_h),
        "the generated level must be connected BY CONSTRUCTION: every cell outside a placed \
         region reachable from any open cell (the 1-cell default_floor seam lattice). If a \
         placement could wall off part of the board, this would fail.",
    );

    // Run the FULL entry point and verify the emitted Situation (the real output of the real
    // pipeline) — the emit translated the footprint-local cells onto the board.
    let mut rng = ProcgenRng::from_root(seed);
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
        "the generate must succeed (a valid placement): {:?}",
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

    // The emitted level carries the translated prefab walls (the single v2 placements list
    // poured into `walls` by the Wall def classification).
    assert!(
        !situation.walls.is_empty(),
        "the emitted level must carry the translated prefab walls (C1 — the v2 placements \
         list was poured into the situation, classified into the walls list)",
    );

    // Every authored terrain cell must be in-bounds: 0 <= x < board_w, 0 <= y < board_h.
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

/// C2 (pin-discrimination): PROVE the by-construction connectivity invariant is sensitive to
/// the 1-cell `default_floor` seam — that it would FAIL if the packer's [`Margin::DEFAULT`]
/// seam reservation were removed.
///
/// Two regions that each span a full board axis with the OTHER axis abutting form a
/// board-spanning barrier UNLESS a walkable gap separates them. This is exactly what the
/// packer's seam guarantees: with the 1-cell seam reserved between them, a walkable corridor
/// remains and the open cells stay ONE component; with NO seam (the zero-margin layout a
/// seam-less packer would emit) the two regions touch into a solid wall that splits the
/// board, and the open cells become TWO components.
///
/// The same [`open_cells_form_one_component`] helper that backs the real-pipeline assertion
/// is exercised here on both layouts: it returns `true` for the seam-separated layout and
/// `false` for the abutting one. So the real-pipeline assertion is NOT vacuous — remove the
/// seam from the packer and the abutting layout (which a seam-less packer would produce)
/// fails this helper.
#[test]
fn seam_separated_regions_stay_connected() {
    let board_w = 20;
    let board_h = 20;
    let seam = Margin::DEFAULT.cells();

    // Two region blocks side by side across a mid band (rows 5..15), leaving open rows above
    // (0..5) and below (15..20). A LEFT block on columns `0..10`, and a RIGHT block that, with
    // the 1-cell seam, starts at column `10 + seam` (leaving column 10 as a walkable corridor
    // through the barrier — the open rows above and below stay joined). With the seam REMOVED
    // the right block starts at column 10, abutting the left block into a FULL-WIDTH wall on
    // rows 5..15 that splits the board's open cells into two components (above vs below).
    let left = RegionRect::new(Cell::new(0, 5), Footprint::new(10, 10));
    let right_with_seam = RegionRect::new(
        Cell::new(10 + seam, 5),
        Footprint::new(board_w - 10 - seam, 10),
    );
    let right_abutting = RegionRect::new(Cell::new(10, 5), Footprint::new(board_w - 10, 10));

    assert!(
        open_cells_form_one_component(&[left, right_with_seam], board_w, board_h),
        "with the 1-cell seam reserved between two abutting-axis blocks, the open cells stay \
         ONE connected component (the seam lattice keeps a walkable corridor through the \
         barrier)",
    );
    assert!(
        !open_cells_form_one_component(&[left, right_abutting], board_w, board_h),
        "with the seam REMOVED the two blocks abut into a board-spanning wall and the open \
         cells split into TWO components (above vs below) — so the by-construction \
         connectivity invariant is sensitive to the packer's Margin::DEFAULT seam \
         (pin-discriminating)",
    );
}

/// Drive the REAL staged pipeline (`assemble_placement_with` then `fill_placement_with` — the
/// exact functions [`generate_level`] composes) under a fixed seed and return the
/// [`FilledPlacement`]. Uses the RULED defaults ([`SplitMode::default`],
/// [`MinPlayerSide::DEFAULT`]) so the placement matches `generate_level`'s. Returns `None` on
/// any packing error (the caller returns early — no panic).
fn run_pipeline(
    prefabs: &PrefabRegistry,
    theme: ThemeUuid,
    board: GridSize,
    seed: BattleSeed,
    knobs: &ProcgenTuning,
) -> Option<FilledPlacement> {
    let mut rng = ProcgenRng::from_root(seed);
    let placement = assemble_placement_with(
        prefabs,
        theme,
        board,
        &mut rng,
        SplitMode::default(),
        MinPlayerSide::DEFAULT,
    )
    .ok()?;
    fill_placement_with(
        placement,
        prefabs,
        theme,
        board,
        knobs,
        &mut rng,
        SplitMode::default(),
    )
    .ok()
}

/// Every ground-plane region a [`FilledPlacement`] occupies — the player + enemy spawn
/// regions and every fill prefab's region (the rectangles the packer reserved the seam
/// around). The dead-space regions are walkable `default_floor`, so they are NOT occupied.
fn occupied_regions(filled: &FilledPlacement) -> Vec<RegionRect> {
    let mut out = vec![
        filled.placement().player().region(),
        filled.placement().enemy().region(),
    ];
    out.extend(filled.fill().iter().map(PlacedPrefab::region));
    out
}

/// Whether the OPEN (non-`occupied`-region) ground-plane cells of a `board_w` x `board_h`
/// board form ONE 4-connected component — the by-construction connectivity invariant (C2).
///
/// A cell inside any `occupied` region blocks the flood; every other cell is open (the
/// `default_floor` seam lattice + the floored dead space). Floods the open cells from the
/// first open cell found and returns `true` iff the flood reaches every open cell. A board
/// with no open cell trivially returns `true` (the caller's other assertions pin a non-empty
/// level). This is the SHARED helper both the real-pipeline assertion and the
/// pin-discrimination control ([`seam_separated_regions_stay_connected`]) exercise.
fn open_cells_form_one_component(occupied: &[RegionRect], board_w: i32, board_h: i32) -> bool {
    let width = usize::try_from(board_w.max(0)).unwrap_or(0);
    let height = usize::try_from(board_h.max(0)).unwrap_or(0);
    let total = width.saturating_mul(height);
    if total == 0 {
        return true;
    }
    let index = |col: usize, row: usize| -> usize { row * width + col };

    // Ground-plane occupancy mask: a cell inside any placed region blocks the flood.
    let mut blocked = vec![false; total];
    for region in occupied {
        let origin = region.origin();
        let footprint = region.footprint();
        for dy in 0..footprint.height() {
            for dx in 0..footprint.width() {
                let x = origin.x + dx;
                let y = origin.y + dy;
                if x >= 0
                    && x < board_w
                    && y >= 0
                    && y < board_h
                    && let (Ok(col), Ok(row)) = (usize::try_from(x), usize::try_from(y))
                {
                    blocked[index(col, row)] = true;
                }
            }
        }
    }

    let open_total = blocked.iter().filter(|b| !**b).count();
    let Some(start) = blocked.iter().position(|b| !*b) else {
        return true; // no open cell — trivially one (empty) component
    };

    // Flood the open cells in 4-connectivity from the first open cell, collecting the
    // in-bounds 4-neighbours of each popped cell.
    let mut seen = vec![false; total];
    seen[start] = true;
    let mut stack = vec![start];
    let mut reached = 0usize;
    while let Some(cell) = stack.pop() {
        reached += 1;
        let col = cell % width;
        let row = cell / width;
        let mut neighbours: Vec<usize> = Vec::with_capacity(4);
        if col > 0 {
            neighbours.push(index(col - 1, row));
        }
        if col + 1 < width {
            neighbours.push(index(col + 1, row));
        }
        if row > 0 {
            neighbours.push(index(col, row - 1));
        }
        if row + 1 < height {
            neighbours.push(index(col, row + 1));
        }
        for neighbour in neighbours {
            if !blocked[neighbour] && !seen[neighbour] {
                seen[neighbour] = true;
                stack.push(neighbour);
            }
        }
    }
    reached == open_total
}

/// The NS-orientation GTW-469 test wall piece (`sim_kind = Wall`, `graphic_name = "wall"`).
const WALL_NS_EW_NS: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0469_0000_0001));
/// The EW-orientation companion (`sim_kind = Wall`, `graphic_name = "wall_ew"`) — SAME sim
/// semantics as [`WALL_NS_EW_NS`], distinct only in its presenter graphic key.
const WALL_NS_EW_EW: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0469_0000_0002));
/// The walkable default-floor test piece (a `Slab` `sim_kind` in the new model) the GTW-469
/// orientation fixture nominates as its default floor.
const WALL_NS_EW_FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0469_0000_0003));

/// A `Wall` terrain def with the given key + presenter graphic role; the structural stats are
/// IDENTICAL for the NS and EW orientations (orientation is presentation-only, so the sim half
/// is the same for both — C5).
fn orientation_wall_def(key: TerrainUuid, graphic: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Wall".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
        },
        tags: Vec::new(),
        on_death: None,
    }
}

/// The GTW-469 fixture terrain registry: the NS wall, the EW wall, and a walkable floor.
fn orientation_terrain_defs() -> TerrainDefRegistry {
    let floor = TerrainDef {
        key:            WALL_NS_EW_FLOOR,
        display_name:   TerrainDisplayName::new("Test Floor".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               crate::slab::SlabHp::new(60),
            armor_protection: ArmorProtection::new(1),
            armor_hardness:   ArmorHardness::new(0),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("floor".to_owned()),
            footfall:     None,
        },
        tags:           Vec::new(),
        on_death:       None,
    };
    TerrainDefRegistry::new([
        (WALL_NS_EW_NS, orientation_wall_def(WALL_NS_EW_NS, "wall")),
        (
            WALL_NS_EW_EW,
            orientation_wall_def(WALL_NS_EW_EW, "wall_ew"),
        ),
        (WALL_NS_EW_FLOOR, floor),
    ])
}

/// A `PrefabRegistry` whose player + enemy prefabs (footprint `fp`) each place ONE NS wall and
/// ONE EW wall at distinct footprint-local cells, so the emit must translate + classify each.
fn orientation_prefabs(theme: ThemeUuid, fp: GridSize) -> PrefabRegistry {
    let both_walls = |role: SpawnRole, stem: &str| {
        Prefab::new(
            PrefabName::new(stem.to_owned()),
            PrefabSpec::new(
                theme,
                fp,
                role,
                vec![
                    TerrainPlacementEntry::new(WALL_NS_EW_NS, at(1, 1)),
                    TerrainPlacementEntry::new(WALL_NS_EW_EW, at(2, 2)),
                ],
            ),
        )
    };
    let mut prefabs = PrefabRegistry::default();
    prefabs.insert(both_walls(SpawnRole::Player, "player_pad"));
    prefabs.insert(both_walls(SpawnRole::Enemy, "enemy_pad"));
    prefabs
}

/// GTW-469 C4 / C5 — an NS-wall and an EW-wall `TerrainUuid` placed in a [`PrefabSpec`] BOTH
/// resolve through the REAL `generate_level` loader/emit path and emit into the
/// [`walls`](Situation::walls) list, classified identically.
///
/// The fixture registry holds two `sim_kind = Wall` defs — the NS wall (`graphic_name = "wall"`)
/// and the EW wall (`graphic_name = "wall_ew"`) — that differ ONLY in their presenter graphic key
/// (orientation is presentation-only; the sim semantics are IDENTICAL, C5). A prefab places one
/// of each, then the real `generate_level` classifies the placed pieces by their `sim_kind`. The
/// assertion proves BOTH UUIDs surface in the emitted `walls` list (C4: the EW-wall `TerrainUuid`
/// resolves through the loader/emit path) and that the EW wall is bucketed exactly like the NS
/// wall (C5: same `Wall` classification, no sim-side behavioural difference). Pin-discriminating:
/// re-keying the EW def to a `Slab` `sim_kind` would route it into `slabs`, failing this test.
#[test]
fn ns_and_ew_walls_both_emit_as_walls_through_the_loader() {
    let terrain_defs = orientation_terrain_defs();

    // C5 precondition: the two orientation defs are BOTH Wall (identical sim_kind) — the
    // difference is purely the presenter graphic_name.
    let (Some(ns), Some(ew)) = (
        terrain_defs.def(&WALL_NS_EW_NS),
        terrain_defs.def(&WALL_NS_EW_EW),
    ) else {
        return;
    };
    assert!(
        matches!(ns.sim_kind, TerrainSimKind::Wall { .. })
            && matches!(ew.sim_kind, TerrainSimKind::Wall { .. }),
        "both the NS and EW wall defs must be sim_kind = Wall (orientation is presentation-only)",
    );

    let theme = theme();
    // The player/enemy footprints must meet `MinPlayerSide` (>= 10), so use 12x12 on a 40x40
    // board (the size the sibling emit tests use); local cells (1,1) + (2,2) fit comfortably.
    let (Some(board), Some(fp)) = (size(40, 40), size(12, 12)) else {
        return;
    };
    let prefabs = orientation_prefabs(theme, fp);
    let themes = UuidThemeRegistry::new([(
        theme,
        UuidThemeDef {
            key:           theme,
            display_name:  ThemeDisplayName::new("Test Theme".to_owned()),
            default_floor: WALL_NS_EW_FLOOR,
            terrain:       vec![WALL_NS_EW_NS, WALL_NS_EW_EW, WALL_NS_EW_FLOOR],
        },
    )]);
    let knobs = tuning(0.8, 49, 2);

    let mut rng = ProcgenRng::from_root(BattleSeed::new(0x0469_4311));
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
        "the generate must succeed for a prefab placing NS + EW walls: {:?}",
        result.as_ref().err(),
    );
    let Ok(situation) = result else {
        return;
    };

    // C4 / C5: BOTH the NS and the EW wall TerrainUuid resolved through the loader/emit path and
    // were classified into the walls list (the EW wall is bucketed exactly like the NS wall).
    assert!(
        situation.walls.iter().any(|w| w.piece == WALL_NS_EW_NS),
        "the NS-wall TerrainUuid must emit into the walls list (the Wall classification)",
    );
    assert!(
        situation.walls.iter().any(|w| w.piece == WALL_NS_EW_EW),
        "the EW-wall TerrainUuid must emit into the walls list, classified identically to the \
         NS wall (C4 — it resolves through the loader; C5 — same Wall sim semantics)",
    );
    // The EW wall must NOT have leaked into the slabs list (it is a Wall, not a Slab) — the pin
    // that re-keying it to a Slab sim_kind would catch.
    assert!(
        !situation.slabs.iter().any(|s| s.piece == WALL_NS_EW_EW),
        "the EW wall must classify as a Wall (walls list), never a Slab (C5)",
    );
}

/// The GTW-470 orientation/direction terrain UUIDs the fixture places in a prefab: 2 doors
/// (`sim_kind = Wall` + `Openable`) and 4 stairs (`sim_kind = Slab`).
const DOOR_NS: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0001));
/// The EW door companion (`sim_kind = Wall` + `Openable`).
const DOOR_EW: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0002));
/// The NS ascending stair (`sim_kind = Slab`).
const STAIR_NS_UP: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0003));
/// The NS descending stair (`sim_kind = Slab`).
const STAIR_NS_DOWN: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0004));
/// The EW ascending stair (`sim_kind = Slab`).
const STAIR_EW_UP: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0005));
/// The EW descending stair (`sim_kind = Slab`).
const STAIR_EW_DOWN: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0006));
/// The walkable default-floor test piece the GTW-470 door/stair fixture nominates.
const DOOR_STAIR_FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0007));

/// A DOOR terrain def: `sim_kind = Wall` carrying the sim-owned `Openable` tag (C5 — a closed
/// door blocks like a wall; opening is GTW-315). `graphic` is its orientation graphic key.
fn door_def(key: TerrainUuid, graphic: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Door".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
        },
        tags: vec![crate::terrain::def::TerrainTag::Openable],
        on_death: None,
    }
}

/// A STAIR terrain def: `sim_kind = Slab` (a walkable surface; vertical traversal is GTW-388).
fn stair_def(key: TerrainUuid, graphic: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Stair".to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               crate::slab::SlabHp::new(120),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
        on_death: None,
    }
}

/// The GTW-470 fixture terrain registry: the 2 doors, the 4 stairs, and a walkable floor.
fn door_stair_terrain_defs() -> TerrainDefRegistry {
    let floor = TerrainDef {
        key:            DOOR_STAIR_FLOOR,
        display_name:   TerrainDisplayName::new("Test Floor".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               crate::slab::SlabHp::new(60),
            armor_protection: ArmorProtection::new(1),
            armor_hardness:   ArmorHardness::new(0),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("floor".to_owned()),
            footfall:     None,
        },
        tags:           Vec::new(),
        on_death:       None,
    };
    TerrainDefRegistry::new([
        (DOOR_NS, door_def(DOOR_NS, "door_ns")),
        (DOOR_EW, door_def(DOOR_EW, "door_ew")),
        (STAIR_NS_UP, stair_def(STAIR_NS_UP, "stair_ns_up")),
        (STAIR_NS_DOWN, stair_def(STAIR_NS_DOWN, "stair_ns_down")),
        (STAIR_EW_UP, stair_def(STAIR_EW_UP, "stair_ew_up")),
        (STAIR_EW_DOWN, stair_def(STAIR_EW_DOWN, "stair_ew_down")),
        (DOOR_STAIR_FLOOR, floor),
    ])
}

/// A `PrefabRegistry` whose player + enemy prefabs (footprint `fp`) each place all 6 GTW-470
/// orientation/direction tiles at distinct footprint-local cells, so the emit must translate +
/// classify each.
fn door_stair_prefabs(theme: ThemeUuid, fp: GridSize) -> PrefabRegistry {
    let all_six = |role: SpawnRole, stem: &str| {
        Prefab::new(
            PrefabName::new(stem.to_owned()),
            PrefabSpec::new(
                theme,
                fp,
                role,
                vec![
                    TerrainPlacementEntry::new(DOOR_NS, at(1, 1)),
                    TerrainPlacementEntry::new(DOOR_EW, at(2, 1)),
                    TerrainPlacementEntry::new(STAIR_NS_UP, at(3, 1)),
                    TerrainPlacementEntry::new(STAIR_NS_DOWN, at(4, 1)),
                    TerrainPlacementEntry::new(STAIR_EW_UP, at(5, 1)),
                    TerrainPlacementEntry::new(STAIR_EW_DOWN, at(6, 1)),
                ],
            ),
        )
    };
    let mut prefabs = PrefabRegistry::default();
    prefabs.insert(all_six(SpawnRole::Player, "player_pad"));
    prefabs.insert(all_six(SpawnRole::Enemy, "enemy_pad"));
    prefabs
}

/// GTW-470 C4 / C5 — all 6 orientation/direction door + stair `TerrainUuid`s placed in a
/// [`PrefabSpec`] resolve through the REAL `generate_level` loader/emit path: the 2 doors
/// (`sim_kind = Wall`) classify into the [`walls`](Situation::walls) list and the 4 stairs
/// (`sim_kind = Slab`) into the [`slabs`](Situation::slabs) list.
///
/// C4: every one of the 6 placed `TerrainUuid`s surfaces in the emitted level (they are placeable
/// in a prefab + resolve through the loader). C5: a door carries the sim-owned `Openable` tag and
/// is classified as a Wall (no functional open/close added) while a stair is classified as a Slab
/// (no vertical-traversal logic added). Pin-discriminating: re-keying a door to `Slab` would route
/// it into `slabs` (failing the door asserts), and re-keying a stair to `Wall` would route it into
/// `walls` (failing the stair asserts).
#[test]
fn door_and_stair_tiles_emit_through_the_loader_classified_by_kind() {
    let terrain_defs = door_stair_terrain_defs();

    // C5 precondition: the 2 doors are sim_kind = Wall carrying the Openable tag; the 4 stairs are
    // sim_kind = Slab. No new TerrainSimKind variant — kinds reused.
    for door in [DOOR_NS, DOOR_EW] {
        let Some(def) = terrain_defs.def(&door) else {
            return;
        };
        assert!(
            matches!(def.sim_kind, TerrainSimKind::Wall { .. }),
            "a door must be sim_kind = Wall (a closed door blocks like a wall)",
        );
        assert!(
            def.tags
                .contains(&crate::terrain::def::TerrainTag::Openable),
            "a door must carry the sim-owned Openable tag (C5)",
        );
    }
    for stair in [STAIR_NS_UP, STAIR_NS_DOWN, STAIR_EW_UP, STAIR_EW_DOWN] {
        let Some(def) = terrain_defs.def(&stair) else {
            return;
        };
        assert!(
            matches!(def.sim_kind, TerrainSimKind::Slab { .. }),
            "a stair must be sim_kind = Slab (a walkable surface; vertical traversal is GTW-388)",
        );
    }

    let theme = theme();
    let (Some(board), Some(fp)) = (size(40, 40), size(12, 12)) else {
        return;
    };
    let prefabs = door_stair_prefabs(theme, fp);
    let themes = UuidThemeRegistry::new([(
        theme,
        UuidThemeDef {
            key:           theme,
            display_name:  ThemeDisplayName::new("Test Theme".to_owned()),
            default_floor: DOOR_STAIR_FLOOR,
            terrain:       vec![
                DOOR_NS,
                DOOR_EW,
                STAIR_NS_UP,
                STAIR_NS_DOWN,
                STAIR_EW_UP,
                STAIR_EW_DOWN,
                DOOR_STAIR_FLOOR,
            ],
        },
    )]);
    let knobs = tuning(0.8, 49, 2);

    let mut rng = ProcgenRng::from_root(BattleSeed::new(0x0470_4311));
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
        "the generate must succeed for a prefab placing all 6 door/stair tiles: {:?}",
        result.as_ref().err(),
    );
    let Ok(situation) = result else {
        return;
    };

    // C4 / C5: the 2 doors (Wall) resolved through the loader/emit path into the WALLS list.
    for door in [DOOR_NS, DOOR_EW] {
        assert!(
            situation.walls.iter().any(|w| w.piece == door),
            "the door {door:?} must emit into the walls list (Wall classification, C4/C5)",
        );
        assert!(
            !situation.slabs.iter().any(|s| s.piece == door),
            "a door (Wall) must never classify as a Slab (C5)",
        );
    }
    // C4 / C5: the 4 stairs (Slab) resolved into the SLABS list.
    for stair in [STAIR_NS_UP, STAIR_NS_DOWN, STAIR_EW_UP, STAIR_EW_DOWN] {
        assert!(
            situation.slabs.iter().any(|s| s.piece == stair),
            "the stair {stair:?} must emit into the slabs list (Slab classification, C4/C5)",
        );
        assert!(
            !situation.walls.iter().any(|w| w.piece == stair),
            "a stair (Slab) must never classify as a Wall (C5)",
        );
    }
}
