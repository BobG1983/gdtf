//! GTW-434: headless behavioral tests for the DEV-ONLY procgen STEP/AUTO visualizer.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] (the real state stack, `UiPlugin`,
//! and — in a debug build — the real `ProcgenVizScenePlugin` wired through `ScenesPlugin`).
//! They seed the persistent `Load` resources the visualizer reads (a theme, a theme+size-only
//! `LoadedSituation`, a [`PrefabRegistry2`] with a player + enemy v2 prefab under the
//! situation's [`ThemeUuid`], and a FIXED `BattleSeed` so the assembled level is reproducible),
//! drive into [`RunningState::DebugProcgenVisualizer`](gdtf_app::test_support::RunningState), and
//! assert on the WORLD + the real visualizer model / entities — never on rendering (the
//! screenshot, C5, is the QA stage).
//!
//! GTW-492 (T07b): the visualizer drives the UUID-keyed v2 procgen pipeline, so the fixture
//! seeds a [`PrefabRegistry2`] of [`Prefab2`] (keyed by the situation's [`ThemeUuid`]) rather
//! than the legacy `PrefabRegistry`.
//!
//! Coverage (C1/C2/C3):
//!
//! - [`step_reveals_one_more_quad`] — a press on the STEP button advances the model's revealed
//!   count by EXACTLY one (C1).
//! - [`auto_reveals_every_quad`] — a press on the AUTO button reveals the WHOLE placement
//!   sequence at once (C1).
//! - [`revealed_quads_carry_role_tints`] — after AUTO, the revealed quad entities carry the
//!   right tints: index 0 (player) = green, index 1 (enemy) = red, the rest = neutral (C3).
//!
//! The whole visualizer is `#[cfg(debug_assertions)]`-gated; these tests run under the dev /
//! gate suite (debug), where the feature is compiled in.

#![cfg(debug_assertions)]

use bevy::{
    ecs::{component::Component, entity::Entity},
    prelude::With,
    state::state::NextState,
    ui::Interaction,
};
use gdtf_app::test_support::{
    AppState, AutoButton, BoardQuad, PrefabQuad, ProcgenViz, ProcgenVizRoot, QuadTint,
    RunningState, StepButton,
};
use gdtf_battle_sim::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab2, PrefabName, PrefabRegistry2,
        PrefabSpecV2, SpawnRole, ThemeUuid,
    },
    rng::BattleSeed,
    situation::Situation,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A bounded budget for driving into the visualizer state.
const BUDGET: u32 = 64;

/// A FIXED injected seed so the assembled level (and therefore the quad count) is reproducible
/// across runs — the visualizer reads a `Res<BattleSeed>` override when present.
const TEST_SEED: u64 = 0x600D_5EED;

/// The canonical test theme key the seeded situation + v2 prefabs share, so the visualizer's
/// theme-keyed candidate lookup resolves the player + enemy prefabs (GTW-492).
const fn viz_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_2434_0000_0001))
}

/// The 30x30x4 board the visualizer assembles into — large enough to fit the 12x12 player +
/// enemy deployment fragments at opposite corners (the `procgen_battle` precedent).
fn viz_board() -> GridSize {
    GridSize::new(GridWidth::new(30), GridHeight::new(30), GridLevels::new(4)).unwrap_or_default()
}

/// A `12x12x1` fragment footprint (the deployment-zone size). Falls back to the default extent
/// on a bad span (it cannot be bad — 12/12/1 are under the maxima).
fn fragment() -> GridSize {
    GridSize::new(GridWidth::new(12), GridHeight::new(12), GridLevels::new(1)).unwrap_or_default()
}

/// Build a [`PrefabRegistry2`] with a player + enemy v2 prefab (each a 12x12 fragment under
/// [`viz_theme`], authoring no placements — the visualizer reads only the placement-quad
/// SEQUENCE, never the per-piece geometry), so the visualizer's `assemble_placement` has a real
/// player- and enemy-role prefab to place. An empty registry would show zero quads and redden
/// the assertions loudly.
fn viz_prefab_registry() -> PrefabRegistry2 {
    let theme = viz_theme();
    let fp = fragment();
    let mut registry = PrefabRegistry2::default();
    registry.insert(Prefab2::new(
        PrefabName::new("player_deployment".to_owned()),
        PrefabSpecV2::new(theme, fp, SpawnRole::Player, Vec::new()),
    ));
    registry.insert(Prefab2::new(
        PrefabName::new("enemy_deployment".to_owned()),
        PrefabSpecV2::new(theme, fp, SpawnRole::Enemy, Vec::new()),
    ));
    registry
}

/// A theme+size [`Situation`] the visualizer reads for its theme + grid-size: the 30x30x4 board
/// under [`viz_theme`], no terrain / gangers (the visualizer never reads those — it only runs
/// the space packer). GTW-492: the visualizer's `build` reads `Situation.theme` (the
/// UUID-keyed `ThemeUuid`) directly, so the theme MUST match the seeded prefabs' theme.
fn viz_situation() -> Situation {
    let mut situation = Situation::new();
    situation.grid_size = viz_board();
    situation.theme = viz_theme();
    situation
}

/// Reads the current [`RunningState`] if active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<bevy::state::state::State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds a headless app driven into [`RunningState::DebugProcgenVisualizer`] with the screen
/// spawned + the model built: seed the theme + registry + situation + seed before the first
/// update, enter `Running` (rests on `Menu`), set the visualizer transition (the cfg-gated
/// button does this in the GUI; here we set it directly, the headless idiom), then pump updates
/// until the model is present.
fn viz_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(viz_prefab_registry());
    // The visualizer reads theme + grid-size from the loaded situation; insert it directly as
    // the persistent resource (the procgen_battle harness precedent).
    app.world_mut()
        .insert_resource(gdtf_app::test_support::LoadedSituation::new(viz_situation()));
    // A fixed seed so the assembled level is reproducible.
    app.world_mut().insert_resource(BattleSeed::new(TEST_SEED));

    // Enter Running (rests on Menu).
    app.update();
    // Drive Menu -> DebugProcgenVisualizer.
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::DebugProcgenVisualizer);
    // Apply the transition + run OnEnter (build model + spawn screen), then settle a frame.
    advance_until(
        &mut app,
        |app| {
            running_state(app) == Some(RunningState::DebugProcgenVisualizer)
                && app.world().get_resource::<ProcgenViz>().is_some()
        },
        BUDGET,
    );
    app.update();
    app
}

/// The single entity carrying marker `M`, if exactly one exists.
fn single_with<M: Component>(app: &mut bevy::app::App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Reads the model's revealed count as a `usize` (0 if the model is absent) — deref-ing the
/// `RevealedCount` newtype the accessor returns.
fn revealed(app: &bevy::app::App) -> usize {
    app.world()
        .get_resource::<ProcgenViz>()
        .map(|model| *model.revealed())
        .unwrap_or_default()
}

/// Reads the model's total quad count (0 if the model is absent).
fn total(app: &bevy::app::App) -> usize {
    app.world()
        .get_resource::<ProcgenViz>()
        .map(|model| model.quads().len())
        .unwrap_or_default()
}

/// Injects a fresh `Interaction::Pressed` on the one button carrying marker `M`, then updates
/// once — driving the real `Changed<Interaction>` control reader (the GTW-420 editor precedent:
/// `MinimalPlugins` has no `ui_focus_system` to clear the injected press, so it persists for the
/// reader the same frame).
fn press_button<M: Component>(app: &mut bevy::app::App) {
    let button = single_with::<M>(app).unwrap_or(Entity::PLACEHOLDER);
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
    app.update();
}

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
