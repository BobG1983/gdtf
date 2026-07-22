//! Fill-pass TERMINATION tests — what STOPS the fill. The loop ends when no more prefab fits
//! (C2, the infinite-loop hazard), when the `Fill` bucket is empty (C2), and — GTW-767 — once
//! coverage reaches the max coverage cap, a stop condition DISTINCT from the density floor and
//! from nothing-fits. What the fill PRODUCES (C1/C3/determinism) lives in
//! [`content`](super::content).

use super::support::{placed_cells, registry_with_fill, size, theme, tuning, tuning_capped};
use crate::{
    procgen::{FilledPlacement, RegionRect, assemble_placement, fill_placement},
    rng::{BattleSeed, ProcgenRng},
};

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

/// GTW-767: the fill ALSO terminates once coverage reaches the max coverage cap — a stop
/// condition DISTINCT from the density floor (C1) and nothing-fits (C2). With the floor set
/// unreachably high (0.99, mirroring the 1.0-floor precedent above so the floor can never be
/// the cause), a run with a low cap (0.20) is compared to an otherwise-identical UNCAPPED
/// (1.0) run from the SAME seed. The cap is set BELOW the coverage this board+seed reaches at
/// exhaustion (the deployment pads alone are 0.18 of the board and the fill adds a little more)
/// so the cap genuinely bites first. Because the cap check draws no RNG, the capped run's
/// placements are a strict PREFIX of the uncapped run's, so:
///   - the capped fill is non-empty (the cap did not choke the fill to nothing),
///   - the capped fill placed STRICTLY FEWER prefabs than the uncapped run — the ONLY reason
///     it can stop earlier is the cap, since the floor is unreachable and a nothing-fits stop
///     would place the identical sequence (equal counts),
///   - the capped coverage reached the 0.20 cap (confirming THIS cap is the cause, not a
///     premature stop),
///   - and the same seed + cap is deterministic (clause 3).
///
/// Discriminating: if the cap were ignored, the capped and uncapped runs would be identical
/// (equal fill counts), failing the strictly-fewer assertion; if the floor drove termination,
/// neither run could stop (the floor is 0.99, unreachable).
#[test]
fn fill_terminates_when_coverage_cap_reached() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    // Fitting fill prefabs so the uncapped run keeps placing past the cap.
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

    // Run the fill for `cap` from a fresh RNG at the fixed seed; the floor (0.99) is unreachable
    // so only the cap — or nothing-fits — can stop the fill early.
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
    // Coverage reached the 0.20 cap, proving the CAP stopped the run (not a premature
    // nothing-fits): placed/board >= 1/5  ⟺  placed * 5 >= board (integer, no float cast).
    assert!(
        placed_cells(&capped) * 5 >= board_cells,
        "the capped run's coverage must reach the 0.20 cap: placed {} of {} board cells",
        placed_cells(&capped),
        board_cells,
    );

    // Clause 3: the same seed + cap produces an IDENTICAL capped filled placement.
    let (Some(a), Some(b)) = (run(0.20), run(0.20)) else {
        return;
    };
    assert_eq!(
        a, b,
        "the same seed + cap must produce an identical capped filled placement (determinism)",
    );
}
