//! GTW-419 — the battlescape procgen LOADING SCREEN + the no-partial-frame guarantee + the
//! transition wiring that COVERS the Generation phase, driven through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine down to
//! `BattleScapeState::Generation`, where the real loading-screen plugin spawns its overlay
//! `OnEnter`, and on to `AnimateIn`, where `DespawnOnExit(Generation)` has torn it down. They
//! cover the contract:
//!
//! - **C1 (transition)** — entry to Generation shows the loading screen (a named
//!   `LoadingScreenRoot` entity EXISTS during Generation); once the sim signals `BattleReady`
//!   the machine ADVANCES to `AnimateIn` and the loading screen is despawned. Pin-discriminating:
//!   a missing loading screen, or an advance NOT gated on `BattleReady`, fails it.
//! - **C2 (no-partial-frame, structural)** — the loading overlay carries a `GlobalZIndex`
//!   strictly ABOVE the battlescape HUD render band (the bottom bar's `10` / the highest HUD
//!   panel) so the opaque overlay occludes any partial level beneath it (the GTW-294 / dropdown
//!   z-assert idiom). The pixel-level no-peek is the QA capture; the z invariant is
//!   headless-assertable.

use bevy::{ecs::entity::Entity, prelude::*, state::state::State, ui::GlobalZIndex};
use gdtf_app::test_support::{
    AppState, BattleScapeState, GameState, LoadingScreenRoot, RunningState,
};
use gdtf_battle_sim::{injuries::InjuryRegistry, tuning::CombatTuning, weapon::WeaponRegistry};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine that
/// never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// The highest battlescape HUD `GlobalZIndex` band — the contextual panel at `20` (the bottom bar
/// is `10`, the on-bar cluster / combat log `11`). The loading screen must sit strictly above this
/// so nothing in the HUD band can ever paint over it (AC2 / C2).
const TOP_HUD_Z: i32 = 20;

/// Reads the current [`BattleScapeState`] if active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// All entities carrying marker `M`.
fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

/// The single entity carrying marker `M`, or `None` if not exactly one.
fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Builds the real stack and seeds the resources the `Load` gate requires (the headless walk has
/// no `AssetServer`), then drives it to `RunningState::Menu` and nudges `Menu → Game` (the menu is
/// the one place that rests). Returns the app resting just below the menu, ready to descend into
/// the battlescape.
fn app_driven_into_game() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the NEW gate-blocking PrefabRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    // GTW-487: the NEW gate-blocking TerrainDefRegistry + UuidThemeRegistry.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());

    let at_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(at_menu, "the walk should reach RunningState::Menu");
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    app
}

/// C1 + C3 (and C2's existence half): on entry to `Generation` the loading screen EXISTS, and
/// once the sim is ready the machine ADVANCES to `AnimateIn` with the loading screen DESPAWNED.
///
/// Pin-discriminating: if `OnEnter(Generation)` did not spawn the screen the first assert fails;
/// if the `Generation → AnimateIn` transition were not gated on the sim's readiness (or the
/// screen were not `DespawnOnExit(Generation)`-scoped), the post-`AnimateIn` despawn assert fails.
#[test]
fn loading_screen_shows_in_generation_then_despawns_on_animate_in() {
    let mut app = app_driven_into_game();

    // Drive down to Generation — the FIRST battlescape sub-state, where the loading screen spawns.
    let at_generation = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::Generation),
        BUDGET,
    );
    assert!(
        at_generation,
        "the walk should descend to BattleScapeState::Generation; last was {:?}",
        battlescape_state(&app),
    );

    // C1: the loading screen is on screen WHILE in Generation.
    assert!(
        single_with::<LoadingScreenRoot>(&mut app).is_some(),
        "exactly one LoadingScreenRoot must exist while in Generation (the loading screen covers \
         the assembly phase)",
    );

    // Advance until the machine has left Generation for AnimateIn (gated on the sim's BattleReady
    // → GenerationComplete → move_on). The battle setup is the empty default situation here, which
    // validates and signals BattleReady, so the gate opens and the machine advances.
    let at_animate_in = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateIn),
        BUDGET,
    );
    assert!(
        at_animate_in,
        "Generation must advance to AnimateIn once the sim signals BattleReady; last was {:?}",
        battlescape_state(&app),
    );

    // C1: the loading screen is gone after the transition (DespawnOnExit(Generation)).
    assert!(
        single_with::<LoadingScreenRoot>(&mut app).is_none(),
        "the loading screen must be despawned after the transition to AnimateIn \
         (DespawnOnExit(Generation))",
    );
}

/// C2 (structural occlusion): the loading overlay carries a `GlobalZIndex` strictly ABOVE the
/// battlescape HUD band, so the opaque overlay occludes any partial level beneath it (the GTW-294
/// / dropdown anti-occlusion z-assert idiom, `bevy-traps.md` #8). The pixel-level no-peek is the
/// QA capture; this is the headless-assertable invariant.
#[test]
fn loading_screen_z_is_above_the_hud_band() {
    let mut app = app_driven_into_game();

    let at_generation = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::Generation),
        BUDGET,
    );
    assert!(
        at_generation,
        "the walk should descend to BattleScapeState::Generation; last was {:?}",
        battlescape_state(&app),
    );

    let root = single_with::<LoadingScreenRoot>(&mut app);
    assert!(
        root.is_some(),
        "exactly one LoadingScreenRoot must exist while in Generation",
    );
    // The `is_some` assert above already failed loudly if the root is missing; bind without a
    // panic (restriction lints deny `panic!`/`unwrap` even in tests).
    let Some(root) = root else {
        return;
    };
    let z = app.world().get::<GlobalZIndex>(root);
    assert!(
        z.is_some_and(|z| z.0 > TOP_HUD_Z),
        "the loading screen root must carry a GlobalZIndex strictly above the HUD band ({TOP_HUD_Z}) \
         so its opaque fill occludes any partial level beneath it (AC2); was {z:?}",
    );

    // Sanity: the GameState is still BattleScape (the screen lives inside the battle layer).
    assert_eq!(
        app.world()
            .get_resource::<State<GameState>>()
            .map(|state| *state.get()),
        Some(GameState::BattleScape),
        "the loading screen is shown inside GameState::BattleScape",
    );
}
