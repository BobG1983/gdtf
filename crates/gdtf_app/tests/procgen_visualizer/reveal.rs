//! Placement-sequence reveal stepping: STEP / AUTO / role tints (C1/C2/C3).

use gdtf_app::test_support::{
    AutoButton, BoardQuad, PrefabQuad, ProcgenVizRoot, QuadTint, RunningState, StepButton,
};

use super::harness::*;

/// C2: the visualizer is reachable + spawns the screen root, and the model built a NON-EMPTY
/// placement sequence (so the STEP / AUTO assertions have quads to reveal). This is the
/// precondition the discriminating tests rely on.
#[test]
fn visualizer_builds_a_placement_sequence() {
    let mut app = viz_app();

    assert_eq!(
        running_state(&app),
        Some(RunningState::DebugProcgenVisualizer),
        "the app must rest in RunningState::DebugProcgenVisualizer after the transition",
    );
    assert!(
        single_with::<ProcgenVizRoot>(&mut app).is_some(),
        "OnEnter must spawn exactly one visualizer screen root (C2)",
    );
    // The single DARK whole-level board quad the light per-prefab quads draw over (C2).
    // `single_with` is `Some` only when EXACTLY one exists — so a dropped board quad (none)
    // or a duplicated one reddens this. The light quads are "tinted quads over a single dark
    // whole-level quad", so that dark quad must be present exactly once.
    assert!(
        single_with::<BoardQuad>(&mut app).is_some(),
        "OnEnter must spawn exactly one dark whole-level board quad (C2)",
    );
    // The real registry has a player + enemy prefab, so the placement sequence is at least the
    // player + enemy quads (fill may add more). A non-empty sequence proves procgen ran.
    assert!(
        total(&app) >= 2,
        "the visualizer must assemble a placement sequence of at least the player + enemy \
         quads (procgen ran against the real registry); total was {}",
        total(&app),
    );
    // Nothing is revealed yet (the reveal starts at zero — STEP / AUTO drive it).
    assert_eq!(revealed(&app), 0, "the reveal count must start at zero");
}

/// C1: a press on the STEP button advances the revealed count by EXACTLY one.
///
/// Pin: a dropped `Changed<Interaction>` read, a no-op step, or an over-advance reddens the
/// `before + 1` assertion.
#[test]
fn step_reveals_one_more_quad() {
    let mut app = viz_app();
    let before = revealed(&app);
    assert_eq!(before, 0, "precondition: nothing revealed at entry");

    press_button::<StepButton>(&mut app);

    assert_eq!(
        revealed(&app),
        before + 1,
        "a STEP press must reveal exactly one more quad (C1)",
    );

    // A second STEP advances by one again (the control is repeatable).
    press_button::<StepButton>(&mut app);
    assert_eq!(
        revealed(&app),
        before + 2,
        "a second STEP press must reveal one more quad again (C1)",
    );
}

/// C1: a press on the AUTO button reveals the WHOLE placement sequence at once.
///
/// Pin: a partial reveal (revealed != total) or a no-op reddens the assertion.
#[test]
fn auto_reveals_every_quad() {
    let mut app = viz_app();
    let total = total(&app);
    assert!(total >= 2, "precondition: a non-empty placement sequence");
    assert_eq!(revealed(&app), 0, "precondition: nothing revealed at entry");

    press_button::<AutoButton>(&mut app);

    assert_eq!(
        revealed(&app),
        total,
        "an AUTO press must reveal the whole placement sequence at once (C1)",
    );
}

/// C3: after AUTO reveals every quad, the revealed quad ENTITIES carry the right tint roles —
/// index 0 (player) = green, index 1 (enemy) = red, every other (fill) = neutral.
///
/// Asserts on the REAL `PrefabQuad` components (not a reimplementation): a wrong projection
/// (player not green, enemy not red, or a fill tinted as a spawn) reddens it.
#[test]
fn revealed_quads_carry_role_tints() {
    let mut app = viz_app();
    press_button::<AutoButton>(&mut app);

    // Collect every per-prefab quad's (index, tint) from the real entities — deref-ing the
    // `RevealIndex` newtype the accessor returns to its `usize` position.
    let mut quads: Vec<(usize, QuadTint)> = {
        let mut q = app.world_mut().query::<&PrefabQuad>();
        q.iter(app.world())
            .map(|quad| (*quad.index(), quad.tint()))
            .collect()
    };
    quads.sort_by_key(|(index, _)| *index);

    assert!(
        quads.len() >= 2,
        "the visualizer must have spawned a quad per placement entry (>= player + enemy); \
         found {}",
        quads.len(),
    );

    // Index 0 is the player spawn (green); index 1 the enemy spawn (red); the rest neutral.
    assert_eq!(
        quads.first().map(|(_, tint)| *tint),
        Some(QuadTint::Player),
        "the FIRST quad (index 0) is the player spawn — tinted green (C3)",
    );
    assert_eq!(
        quads.get(1).map(|(_, tint)| *tint),
        Some(QuadTint::Enemy),
        "the SECOND quad (index 1) is the enemy spawn — tinted red (C3)",
    );
    for (index, tint) in quads.iter().skip(2) {
        assert_eq!(
            *tint,
            QuadTint::Neutral,
            "every fill quad (index {index}) carries the neutral tint (C3)",
        );
    }
}
