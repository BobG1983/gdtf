//! Driving the regenerated model: STEP / AUTO after Generate, gang quad labels, and
//! empty-registry safety (C4/C5).

use bevy::state::state::NextState;
use gdtf_app::test_support::{
    AppState, AutoButton, EnemyGangDropdown, GenerateButton, HeightField, PlayerGangDropdown,
    PrefabQuad, ProcgenViz, RunningState, StepButton, WidthField,
};
use gdtf_battle_sim::{GangRegistry, rng::BattleSeed};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

use super::{form::*, harness::*};

/// The display label of the model's quad at `index`, if any.
fn quad_label(app: &bevy::app::App, index: usize) -> Option<String> {
    app.world()
        .get_resource::<ProcgenViz>()
        .and_then(|model| model.quad_label_at(index))
}

/// C5: after a Generate, the EXISTING STEP / AUTO controls step through the NEW model — STEP
/// reveals one more, AUTO reveals them all.
///
/// Pin: a Generate that does not reset the reveal, or quads that were not re-spawned for the new
/// model, reddens (STEP / AUTO would act on a stale model or stale entities).
#[test]
fn step_and_auto_drive_the_regenerated_model() {
    let mut app = viz_app();
    // Regenerate to a different board (still large enough to fit the 12x12 fragments) so the new
    // model is distinct from the OnEnter one.
    commit_u8_field::<WidthField>(&mut app, 40);
    commit_u8_field::<HeightField>(&mut app, 40);
    press_button::<GenerateButton>(&mut app);

    let total_after = total(&app);
    assert!(
        total_after >= 2,
        "the regenerated model has quads to reveal"
    );
    assert_eq!(revealed(&app), 0, "Generate reset the reveal to zero (C5)");

    press_button::<StepButton>(&mut app);
    assert_eq!(
        revealed(&app),
        1,
        "STEP reveals one more quad on the NEW model (C5)",
    );

    press_button::<AutoButton>(&mut app);
    assert_eq!(
        revealed(&app),
        total_after,
        "AUTO reveals every quad of the NEW model (C5)",
    );

    // The re-spawned per-prefab quad entities match the new model's quad count (re-spawn ran).
    let mut q = app.world_mut().query::<&PrefabQuad>();
    let spawned = q.iter(app.world()).count();
    assert_eq!(
        spawned, total_after,
        "the per-prefab quad ENTITIES were re-spawned for the new model (C5)",
    );
}

/// C4: chosen player / enemy gangs LABEL the player (index 0) and enemy (index 1) deployment
/// quads with the gang name + member count, after a Generate.
///
/// Pin: a gang that does not annotate the quad, the wrong quad annotated, or a missing member
/// count, reddens. Asserts the label CONTAINS the gang name (a structural fact, not an exact
/// layout string).
#[test]
fn chosen_gangs_label_the_deployment_quads() {
    let mut app = viz_app();
    select_gang::<PlayerGangDropdown>(&mut app, player_gang_name());
    select_gang::<EnemyGangDropdown>(&mut app, enemy_gang_name());

    let Some(config) = config(&app) else {
        unreachable!("the VizConfig must be present (C4)")
    };
    assert_eq!(
        config.player_gang(),
        Some(&player_gang_name()),
        "selecting a player gang updates the config (C4)",
    );

    press_button::<GenerateButton>(&mut app);

    let player_label = quad_label(&app, 0).unwrap_or_default();
    let enemy_label = quad_label(&app, 1).unwrap_or_default();
    assert!(
        player_label.contains("goliaths") && player_label.contains('2'),
        "the player deployment quad (index 0) must be labelled with the chosen gang name + its \
         member count (C4); was {player_label:?}",
    );
    assert!(
        enemy_label.contains("eschers") && enemy_label.contains('1'),
        "the enemy deployment quad (index 1) must be labelled with the chosen gang name + its \
         member count (C4); was {enemy_label:?}",
    );
}

/// C4: an EMPTY gang registry does not panic — the visualizer is still reachable, Generate still
/// works, and the deployment quads fall back to prefab-name labels (no chosen gang).
#[test]
fn empty_gang_registry_does_not_panic() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(viz_prefab_registry());
    app.world_mut().insert_resource(viz_theme_registry());
    // EMPTY gang registry — the no-gang fallback (C4).
    app.world_mut().insert_resource(GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_app::test_support::LoadedSituation::new(viz_situation()));
    app.world_mut().insert_resource(BattleSeed::new(TEST_SEED));

    app.update();
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::DebugProcgenVisualizer);
    advance_until(
        &mut app,
        |app| {
            running_state(app) == Some(RunningState::DebugProcgenVisualizer)
                && app.world().get_resource::<ProcgenViz>().is_some()
        },
        BUDGET,
    );
    app.update();

    // Generate works with no gangs chosen + an empty registry (never panics, C4).
    press_button::<GenerateButton>(&mut app);
    assert!(
        total(&app) >= 2,
        "the visualizer regenerates with an empty gang registry (C4 no-gang fallback)",
    );
    // The player quad falls back to its prefab name (the player_deployment prefab), not a gang.
    let player_label = quad_label(&app, 0).unwrap_or_default();
    assert!(
        player_label.contains("player_deployment"),
        "with no chosen gang the player quad labels by prefab name (C4); was {player_label:?}",
    );
}
