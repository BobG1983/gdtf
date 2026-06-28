//! End-to-end assembler tests (GTW-424 C1/C2/C3): anchor selection, opposite-side fit,
//! determinism, the OQ-5 minimum size, the OQ-4 connectivity assertion (holds for a valid
//! placement; fails closed on an impossible one), and the fail-closed errors.

use crate::{
    level::{
        EdgeOpening, GridHeight, GridLevels, GridSize, GridWidth, LevelTheme, Prefab, PrefabName,
        PrefabRegistry, PrefabSpec, SpawnRole,
    },
    metric::{Cell, CellLevel, Level},
    procgen::{
        Anchor, Footprint, MinPlayerSide, PackingError, RegionCount, RegionRect, SplitMode,
        assemble_placement, assemble_placement_with, count_seam_reachable,
    },
    rng::{BattleSeed, ProcgenRng},
};

/// A `(cell, level)` on level 0 (a tiny helper).
fn at(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A footprint / board `GridSize` (clamped; `None` returns early — never panics).
fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

/// One validated prefab of `role` at footprint `fp`, with one edge opening (so it
/// validates). `None` if the spec is invalid.
fn prefab(theme: LevelTheme, fp: GridSize, role: SpawnRole, stem: &str) -> Option<Prefab> {
    let spec = PrefabSpec {
        theme,
        size: fp,
        spawn_role: role,
        edge_openings: vec![EdgeOpening::new(at(0, 0))],
        ..PrefabSpec::default()
    };
    Prefab::new(PrefabName::new(stem.to_owned()), spec).ok()
}

/// A registry with one player + one enemy prefab at the given fragment footprints. `None`
/// if any size is invalid.
fn registry(theme: LevelTheme, player_fp: GridSize, enemy_fp: GridSize) -> Option<PrefabRegistry> {
    let mut r = PrefabRegistry::default();
    r.insert(prefab(theme, player_fp, SpawnRole::Player, "player_pad")?);
    r.insert(prefab(theme, enemy_fp, SpawnRole::Enemy, "enemy_pad")?);
    Some(r)
}

/// C1/C2: a valid registry places a `>= 10x10` player-spawn at one of the four anchors and
/// an enemy-spawn at the strict opposite that FITS, connectivity assertion holding.
#[test]
fn places_player_and_opposite_enemy() {
    let theme = LevelTheme::IndustrialHive;
    // A 40x40 board with 12x12 deployment fragments — both fit at opposite anchors with a
    // seam, with plenty of floor between.
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(registry) = registry(theme, player_fp, enemy_fp) else {
        return;
    };

    let mut rng = ProcgenRng::from_root(BattleSeed::new(11));
    let result = assemble_placement(&registry, theme, board, &mut rng);
    assert!(
        result.is_ok(),
        "expected a valid placement, got {:?}",
        result.as_ref().err()
    );
    let Ok(placement) = result else {
        return;
    };

    let player_anchor = placement.player().anchor();
    assert!(
        Anchor::PLAYER_ANCHORS.contains(&player_anchor),
        "player anchor {player_anchor:?} must be one of the four (C1)",
    );
    assert_eq!(
        placement.enemy().anchor(),
        player_anchor.opposite(),
        "enemy anchor must be the strict opposite of the player anchor (OQ-2)",
    );
    assert!(
        !placement
            .player()
            .region()
            .intersects(placement.enemy().region()),
        "player and enemy regions must not overlap (C2)",
    );
    // The chosen player fragment cleared the OQ-5 minimum side.
    assert!(
        Footprint::of(placement.player().prefab().spec().size).min_side()
            >= MinPlayerSide::DEFAULT.cells(),
        "the placed player fragment must clear the 10-cell minimum (OQ-5)",
    );
}

/// C3 (determinism): the same seed produces an identical placement. The RNG draw order is
/// fixed (one player-anchor draw; no draw for the enemy side or prefab choice).
#[test]
fn placement_is_deterministic_under_a_seed() {
    let theme = LevelTheme::Underhive;
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(registry) = registry(theme, player_fp, enemy_fp) else {
        return;
    };

    let seed = BattleSeed::new(0xABCD_1234);
    let mut a = ProcgenRng::from_root(seed);
    let mut b = ProcgenRng::from_root(seed);
    let pa = assemble_placement(&registry, theme, board, &mut a);
    let pb = assemble_placement(&registry, theme, board, &mut b);
    assert_eq!(
        pa, pb,
        "the same seed must produce an identical placement (determinism, C3)",
    );
}

/// OQ-5: with ONLY an undersize player prefab (below `~10x10`), the assembler rejects fail
/// closed with the typed error.
///
/// Discriminating: an 8x8 player footprint (below the 10 floor) must error; the 12x12 in
/// the success test above is accepted. The pair pins the floor both ways.
#[test]
fn undersize_player_footprint_is_rejected_fail_closed() {
    let theme = LevelTheme::IndustrialHive;
    let (Some(board), Some(small), Some(enemy_fp)) = (size(40, 40), size(8, 8), size(12, 12))
    else {
        return;
    };
    let Some(registry) = registry(theme, small, enemy_fp) else {
        return;
    };
    let mut rng = ProcgenRng::from_root(BattleSeed::new(3));
    let result = assemble_placement(&registry, theme, board, &mut rng);
    assert!(
        matches!(result, Err(PackingError::PlayerFootprintTooSmall { .. })),
        "expected PlayerFootprintTooSmall, got {result:?}",
    );
    if let Err(PackingError::PlayerFootprintTooSmall { min_side, .. }) = result {
        assert_eq!(min_side, MinPlayerSide::DEFAULT, "the 10-cell floor");
    }
}

/// Fail-closed: an empty registry (no player prefab) errors with `NoPrefabForRole(Player)`
/// rather than panicking.
#[test]
fn missing_prefab_is_rejected_fail_closed() {
    let theme = LevelTheme::SumpWaste;
    let Some(board) = size(40, 40) else {
        return;
    };
    let registry = PrefabRegistry::default();
    let mut rng = ProcgenRng::from_root(BattleSeed::new(99));
    let result = assemble_placement(&registry, theme, board, &mut rng);
    assert!(
        matches!(
            result,
            Err(PackingError::NoPrefabForRole {
                role: SpawnRole::Player,
                ..
            })
        ),
        "an empty registry must fail closed with NoPrefabForRole(Player), got {result:?}",
    );
}

/// C3 (connectivity holds, A/B): a valid placement passes the OQ-4 connectivity assertion
/// under the guillotine split too (the assertion is split-mode independent).
#[test]
fn connectivity_holds_under_guillotine_split() {
    let theme = LevelTheme::IndustrialHive;
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(registry) = registry(theme, player_fp, enemy_fp) else {
        return;
    };
    let mut rng = ProcgenRng::from_root(BattleSeed::new(5));
    let result = assemble_placement_with(
        &registry,
        theme,
        board,
        &mut rng,
        SplitMode::Guillotine,
        MinPlayerSide::DEFAULT,
    );
    assert!(
        result.is_ok(),
        "a valid placement must pass the connectivity assertion under guillotine: {:?}",
        result.err(),
    );
}

/// C2 (opposite fit fail-closed): an enemy fragment too large for the strict-opposite
/// region (once the player fragment + seam are placed) is rejected with the typed
/// `FootprintDoesNotFit` rather than placed overlapping or panicking.
///
/// A 20x20 board with a 12x12 player fragment leaves a strip too narrow for a 12x12 enemy
/// fragment at the opposite anchor (12 + seam + 12 > 20), so the enemy fails to fit.
#[test]
fn enemy_footprint_that_does_not_fit_is_rejected() {
    let theme = LevelTheme::IndustrialHive;
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(20, 20), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(registry) = registry(theme, player_fp, enemy_fp) else {
        return;
    };
    let mut rng = ProcgenRng::from_root(BattleSeed::new(1));
    let result = assemble_placement(&registry, theme, board, &mut rng);
    assert!(
        matches!(result, Err(PackingError::FootprintDoesNotFit { .. })),
        "an enemy fragment too large for the opposite region must fail closed with \
         FootprintDoesNotFit, got {result:?}",
    );
}

/// C3 (connectivity HOLDS): two seam-separated opposite-corner regions on a board are all
/// seam-reachable from the player region (`reached == total`).
///
/// Discriminating sibling of the fail-closed test below: a valid pair reaches 2 of 2.
#[test]
fn connectivity_holds_for_valid_opposite_placement() {
    let Some(board_size) = size(40, 40) else {
        return;
    };
    let board = RegionRect::board(board_size);
    let player = board.place_at_anchor(Anchor::BottomLeft, Footprint::new(12, 12));
    let enemy = board.place_at_anchor(Anchor::TopRight, Footprint::new(12, 12));
    let (reached, total) = count_seam_reachable(&[player, enemy], board);
    assert_eq!(
        (reached, total),
        (RegionCount::new(2), RegionCount::new(2)),
        "both opposite-corner regions must be seam-reachable (connectivity holds)",
    );
}

/// C3 (connectivity FAILS CLOSED): a full-board-width wall region strictly between the
/// player region and another walls them apart — the connectivity flood reaches only 1 of
/// 3, so the OQ-4 assertion would reject it (NEVER repair).
///
/// Discriminating: the wall region spans the full board width at mid-height, so no floor
/// corridor joins the bottom region to the top region. Removing the full-axis wall check
/// in `seam_adjacent` would make this reach 3 of 3 and the test fail — pinning the
/// assertion's honesty.
#[test]
fn connectivity_fails_closed_for_walled_off_placement() {
    let Some(board_size) = size(40, 40) else {
        return;
    };
    let board = RegionRect::board(board_size);
    // region 0: a 12x12 in the bottom-left (the player region, the flood root).
    let bottom = RegionRect::new(Cell::new(0, 0), Footprint::new(12, 12));
    // region 1: a FULL-WIDTH wall band across the middle (y 18..20), separating top/bottom.
    let wall = RegionRect::new(Cell::new(0, 18), Footprint::new(40, 2));
    // region 2: a 12x12 in the top-left, on the far side of the wall.
    let top = RegionRect::new(Cell::new(0, 28), Footprint::new(12, 12));
    let (reached, total) = count_seam_reachable(&[bottom, wall, top], board);
    assert!(
        reached < total,
        "a full-width wall must disconnect the top region — reached {reached:?} of {total:?}",
    );
}
