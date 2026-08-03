use super::support::*;
use crate::{
    procgen::generate_level,
    rng::{BattleSeed, ProcgenRng},
    situation::Situation,
};

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

    let Some(c) = run(BattleSeed::new(0xD1FF_4311)) else {
        return;
    };
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
