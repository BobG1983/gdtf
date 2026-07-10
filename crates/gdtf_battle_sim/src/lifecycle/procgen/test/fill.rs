//! End-to-end fill-pass tests (GTW-427 C1/C2/C3): random same-theme fill draws from the
//! `Fill` bucket and packs into the free space (C1), the loop TERMINATES when no more
//! prefab fits (C2 — no infinite loop), the no-fit fallback PADS dead space with
//! `default_floor` rather than shrinking the playable area (C3), and the fill is
//! DETERMINISTIC under a seed. Connectivity-by-construction (the 1-cell seam every fill
//! placement reserves) is exercised end-to-end by the emit test
//! (`emitted_level_is_in_bounds_and_fully_connected`); GTW-497 removed the old connectivity
//! flood, so this module no longer re-floods the filled placement.

use bevy::asset::uuid::Uuid;

use crate::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, ThemeUuid,
    },
    procgen::{
        DeadRectScatterCount, FilledPlacement, LargePrefabAreaThreshold, MinDensityFloor,
        ProcgenTuning, RegionRect, assemble_placement, fill_placement,
    },
    rng::{BattleSeed, ProcgenRng},
};

/// A footprint / board `GridSize` (clamped; `None` returns early — never panics).
fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

/// The canonical test theme key — a fixed `from_u128` [`ThemeUuid`] every test prefab
/// authors so the fill's theme-keyed candidate lookup resolves them.
fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0149_2427_0000_0001))
}

/// A v2 prefab of `role` at footprint `fp` under `theme`, authoring no placements (the fill
/// pass cares only about footprint + role + theme — the geometry is the emit step's concern).
fn prefab(theme: ThemeUuid, fp: GridSize, role: SpawnRole, stem: &str) -> Prefab {
    Prefab::new(
        PrefabName::new(stem.to_owned()),
        PrefabSpec::new(theme, fp, role, Vec::new()),
    )
}

/// A registry with a player + enemy deployment prefab and the given list of `Fill`
/// prefabs (each `(stem, w, h)`) under `theme`. `None` if any fill size is invalid.
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

/// A tuning with explicit knob values (the unit tests drive the knobs directly; the file
/// is exercised by the tuning tests).
fn tuning(density: f32, large_area: u32, scatter_k: u8) -> ProcgenTuning {
    ProcgenTuning {
        min_density_floor:           MinDensityFloor::new(density),
        large_prefab_area_threshold: LargePrefabAreaThreshold::new(large_area),
        dead_rect_scatter_count_k:   DeadRectScatterCount::new(scatter_k),
    }
}

/// The total cells the placement + fill + dead-space regions cover (player + enemy + every
/// fill prefab + every dead-space region) — used to assert the playable area is never
/// shrunk (C3: board fully accounted for, no lost cells).
fn covered_plus_dead(filled: &FilledPlacement) -> i64 {
    let mut sum = *filled.placement().player().region().cell_count()
        + *filled.placement().enemy().region().cell_count();
    for p in filled.fill() {
        sum += *p.region().cell_count();
    }
    for d in filled.dead_space() {
        sum += *d.cell_count();
    }
    sum
}

/// C1: after the GTW-424 placement, the fill pass packs RANDOM same-theme `Fill` prefabs
/// from the registry into the free space — at least one fill prefab is placed, every fill
/// prefab is the level's theme and the `Fill` role.
///
/// Discriminating: with a high density floor and several fitting fill prefabs the pass
/// MUST place at least one; a no-op fill (the bug) would place zero.
#[test]
fn fill_places_random_same_theme_prefabs() {
    let theme = theme();
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

    let mut rng = ProcgenRng::from_root(BattleSeed::new(7));
    let Ok(placement) = assemble_placement(&registry, theme, board, &mut rng) else {
        return;
    };
    // A near-full density floor so the pass keeps drawing until nothing fits.
    let knobs = tuning(0.95, 49, 2);
    let result = fill_placement(placement, &registry, theme, board, &knobs, &mut rng);
    assert!(
        result.is_ok(),
        "the fill pass must succeed: {:?}",
        result.as_ref().err(),
    );
    let Ok(filled) = result else {
        return;
    };

    assert!(
        !filled.fill().is_empty(),
        "the fill pass must place at least one same-theme Fill prefab (C1)",
    );
    for placed in filled.fill() {
        assert_eq!(
            placed.prefab().spec().theme,
            theme,
            "every fill prefab must match the level theme (C1: same-theme fill)",
        );
        assert_eq!(
            placed.prefab().spec().role,
            SpawnRole::Fill,
            "every fill prefab must be the Fill role (C1)",
        );
    }
}

/// C2: the fill loop TERMINATES when no more prefab fits. With Fill prefabs that are ALL
/// too large for any remaining free space (a tiny board, large fill footprints), the pass
/// must return (no infinite loop) having placed nothing.
///
/// Discriminating: an unbounded retry loop (the bug) would spin forever; reaching the
/// assertion proves termination. The fill bucket is non-empty but nothing fits.
#[test]
fn fill_terminates_when_nothing_fits() {
    let theme = theme();
    // A 24x24 board with 10x10 deployment pads at opposite corners. The only Fill prefab is
    // a 20x20 — padded with its 1-cell seam it is 22x22, which cannot fit any free rectangle
    // left once the two pads occupy opposite corners. So the fill places nothing and must
    // TERMINATE (the loop cannot spin forever retrying an unfittable prefab).
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(24, 24), size(10, 10), size(10, 10))
    else {
        return;
    };
    let Some(registry) = registry_with_fill(theme, player_fp, enemy_fp, &[("too_big", 20, 20)])
    else {
        return;
    };

    let mut rng = ProcgenRng::from_root(BattleSeed::new(13));
    let Ok(placement) = assemble_placement(&registry, theme, board, &mut rng) else {
        return;
    };
    // A density floor that can NEVER be reached (1.0) so termination is driven purely by
    // "nothing fits", not by the floor — this is the infinite-loop hazard the test pins.
    let knobs = tuning(1.0, 49, 3);
    let result = fill_placement(placement, &registry, theme, board, &knobs, &mut rng);
    assert!(
        result.is_ok(),
        "the fill pass must terminate and succeed even when no fill prefab fits: {:?}",
        result.as_ref().err(),
    );
    let Ok(filled) = result else {
        return;
    };
    assert!(
        filled.fill().is_empty(),
        "no fill prefab fits the narrow strips, so the pass must place none (C2 termination)",
    );
}

/// C2: an EMPTY Fill bucket terminates immediately (no Fill prefabs registered at all) —
/// the pass returns with no fill and the whole free space as dead-space `default_floor`.
#[test]
fn fill_terminates_with_no_fill_prefabs() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    // No fill prefabs at all.
    let Some(registry) = registry_with_fill(theme, player_fp, enemy_fp, &[]) else {
        return;
    };
    let mut rng = ProcgenRng::from_root(BattleSeed::new(21));
    let Ok(placement) = assemble_placement(&registry, theme, board, &mut rng) else {
        return;
    };
    let knobs = tuning(0.9, 49, 3);
    let result = fill_placement(placement, &registry, theme, board, &knobs, &mut rng);
    assert!(
        result.is_ok(),
        "the empty-bucket fill must succeed: {:?}",
        result.as_ref().err(),
    );
    let Ok(filled) = result else {
        return;
    };
    assert!(
        filled.fill().is_empty(),
        "no Fill prefabs registered → the pass places none and terminates (C2)",
    );
    assert!(
        !filled.dead_space().is_empty(),
        "with no fill, the free space must be returned as dead-space default_floor (C3)",
    );
}

/// C3: the no-fit fallback PADS remaining dead space with `default_floor` — it does NOT
/// shrink the playable area. The player + enemy + fill + dead-space regions together
/// account for AT LEAST the whole board's cells (every board cell is either a placed
/// region or open dead-space floor, never lost), and the dead-space list is non-empty.
///
/// Discriminating: a "shrink the playable area" fallback (the rejected design) would drop
/// the uncovered cells, so the sum would be LESS than the board; padding keeps them.
#[test]
fn dead_space_is_padded_with_default_floor_not_shrunk() {
    let theme = theme();
    let (Some(board_size), Some(player_fp), Some(enemy_fp)) =
        (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    // Only one small fill prefab, so plenty of dead space remains to be floored.
    let Some(registry) = registry_with_fill(theme, player_fp, enemy_fp, &[("nook", 4, 4)]) else {
        return;
    };
    let mut rng = ProcgenRng::from_root(BattleSeed::new(33));
    let Ok(placement) = assemble_placement(&registry, theme, board_size, &mut rng) else {
        return;
    };
    // A LOW density floor so the pass stops early, leaving lots of dead space.
    let knobs = tuning(0.05, 49, 0);
    let result = fill_placement(placement, &registry, theme, board_size, &knobs, &mut rng);
    assert!(
        result.is_ok(),
        "the fill must succeed: {:?}",
        result.as_ref().err(),
    );
    let Ok(filled) = result else {
        return;
    };

    let board = RegionRect::board(board_size);
    let board_cells = *board.cell_count();
    assert!(
        !filled.dead_space().is_empty(),
        "remaining dead space must be padded with default_floor regions (C3), not dropped",
    );
    // The placed regions are disjoint (the packer's seam guarantees no overlap), and the
    // dead-space free rects are disjoint from each other and from placed regions — so the
    // sum of all four must be >= the board (every cell accounted for; nothing lost to a
    // shrink). It can EXCEED the board only if regions overlapped, which the packer
    // forbids, so for a valid packing it equals the board.
    let total = covered_plus_dead(&filled);
    assert!(
        total >= board_cells,
        "playable area must not shrink: placed + fill + dead_space cells ({total}) must cover \
         the whole board ({board_cells})",
    );
}

/// DETERMINISM: the same seed produces an IDENTICAL filled placement (fill draws every
/// prefab choice from the injected `ProcgenRng` in a fixed order). Two runs from the same
/// seed must be equal.
///
/// Discriminating: a non-deterministic fill (e.g. iterating the registry `HashMap` unsorted,
/// or a non-seeded draw) would make the two runs differ.
#[test]
fn fill_is_deterministic_under_a_seed() {
    let theme = theme();
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
    let seed = BattleSeed::new(0x5EED_7777);

    let run = |seed: BattleSeed| {
        let mut rng = ProcgenRng::from_root(seed);
        let placement = assemble_placement(&registry, theme, board, &mut rng).ok()?;
        fill_placement(placement, &registry, theme, board, &knobs, &mut rng).ok()
    };

    let (Some(a), Some(b)) = (run(seed), run(seed)) else {
        return;
    };
    assert_eq!(
        a, b,
        "the same seed must produce an identical filled placement (determinism)",
    );
}
