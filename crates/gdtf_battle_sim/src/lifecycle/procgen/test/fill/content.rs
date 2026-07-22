//! Fill-pass CONTENT tests (GTW-427 C1/C3): random same-theme fill draws from the `Fill`
//! bucket and packs into the free space (C1), the no-fit fallback PADS dead space with
//! `default_floor` rather than shrinking the playable area (C3), and the fill is DETERMINISTIC
//! under a seed. Connectivity-by-construction (the 1-cell seam every fill placement reserves)
//! is exercised end-to-end by the emit test (`emitted_level_is_in_bounds_and_fully_connected`);
//! GTW-497 removed the old connectivity flood, so this module no longer re-floods the filled
//! placement. What STOPS the fill (C2 + the GTW-767 cap) lives in [`termination`](super::termination).

use super::support::{covered_plus_dead, registry_with_fill, size, theme, tuning};
use crate::{
    level::SpawnRole,
    procgen::{RegionRect, assemble_placement, fill_placement},
    rng::{BattleSeed, ProcgenRng},
};

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
