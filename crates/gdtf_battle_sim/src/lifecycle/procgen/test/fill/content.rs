use super::support::{covered_plus_dead, registry_with_fill, size, theme, tuning};
use crate::{
    level::SpawnRole,
    procgen::{RegionRect, assemble_placement, fill_placement},
    rng::{BattleSeed, ProcgenRng},
};

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

#[test]
fn dead_space_is_padded_with_default_floor_not_shrunk() {
    let theme = theme();
    let (Some(board_size), Some(player_fp), Some(enemy_fp)) =
        (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(registry) = registry_with_fill(theme, player_fp, enemy_fp, &[("nook", 4, 4)]) else {
        return;
    };
    let mut rng = ProcgenRng::from_root(BattleSeed::new(33));
    let Ok(placement) = assemble_placement(&registry, theme, board_size, &mut rng) else {
        return;
    };
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
    let total = covered_plus_dead(&filled);
    assert!(
        total >= board_cells,
        "playable area must not shrink: placed + fill + dead_space cells ({total}) must cover \
         the whole board ({board_cells})",
    );
}

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
