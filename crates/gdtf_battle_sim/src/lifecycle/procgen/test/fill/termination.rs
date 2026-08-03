use super::support::{placed_cells, registry_with_fill, size, theme, tuning, tuning_capped};
use crate::{
    procgen::{FilledPlacement, RegionRect, assemble_placement, fill_placement},
    rng::{BattleSeed, ProcgenRng},
};

#[test]
fn fill_terminates_when_nothing_fits() {
    let theme = theme();
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

#[test]
fn fill_terminates_with_no_fill_prefabs() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
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

#[test]
fn fill_terminates_when_coverage_cap_reached() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(registry) = registry_with_fill(
        theme,
        player_fp,
        enemy_fp,
        &[("hall", 8, 8), ("room", 6, 6), ("nook", 4, 4)],
    ) else {
        return;
    };
    let seed = BattleSeed::new(0x0CA9_0767);
    let board_cells = *RegionRect::board(board).cell_count();

    let run = |cap: f32| -> Option<FilledPlacement> {
        let mut rng = ProcgenRng::from_root(seed);
        let placement = assemble_placement(&registry, theme, board, &mut rng).ok()?;
        let knobs = tuning_capped(0.99, 49, 3, cap);
        fill_placement(placement, &registry, theme, board, &knobs, &mut rng).ok()
    };

    let (Some(capped), Some(uncapped)) = (run(0.20), run(1.0)) else {
        return;
    };

    assert!(
        !capped.fill().is_empty(),
        "the capped fill must still place prefabs before the cap bites (not choke to nothing)",
    );
    assert!(
        capped.fill().len() < uncapped.fill().len(),
        "the cap must stop the fill EARLIER than exhaustion: capped placed {} prefabs, uncapped \
         {} — equal counts would mean the cap did nothing",
        capped.fill().len(),
        uncapped.fill().len(),
    );
    assert!(
        placed_cells(&capped) * 5 >= board_cells,
        "the capped run's coverage must reach the 0.20 cap: placed {} of {} board cells",
        placed_cells(&capped),
        board_cells,
    );

    let (Some(a), Some(b)) = (run(0.20), run(0.20)) else {
        return;
    };
    assert_eq!(
        a, b,
        "the same seed + cap must produce an identical capped filled placement (determinism)",
    );
}
