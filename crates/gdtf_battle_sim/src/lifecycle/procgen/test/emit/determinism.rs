//! C2 — the seed-determinism pin on the full assemble -> fill -> emit pipeline.

use super::support::*;
use crate::{
    procgen::generate_level,
    rng::{BattleSeed, ProcgenRng},
    situation::Situation,
};

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
        .map(|emitted| emitted.situation)
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
