//! The walk-to-Generation app driver + the shared situation fixture.

use bevy::state::state::State;
use gdtf_app::test_support::{BattleScapeState, LoadedSituation, RunningState};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    test_support::{
        SituationBuilder, ganger_at, key, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::CombatTuning,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk down into the battlescape (each
/// leaf scene spends a couple of `FixedUpdate` ticks plus its transition
/// propagation), but bounded so a machine that never reaches the predicate fails
/// instead of hanging.
pub(crate) const BUDGET: u32 = 96;

/// A valid two-ganger fixture situation (no cover / slabs / links needed — a
/// link-free situation validates trivially), built over the central
/// [`SituationBuilder`](gdtf_battle_sim::test_support::SituationBuilder) +
/// [`ganger_at`](gdtf_battle_sim::test_support::ganger_at).
pub(crate) fn two_ganger_situation() -> Situation {
    SituationBuilder::new()
        .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
        .build()
}

/// Reads the current [`BattleScapeState`] if it is active.
pub(crate) fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
pub(crate) fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Stands in for the player at the menu (it no longer auto-advances, GTW-121):
/// advances until [`RunningState::Menu`] rests, then queues `Menu → Options`.
pub(crate) fn drive_past_menu(app: &mut bevy::app::App) -> bool {
    let reached = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if reached {
        app.world_mut()
            .resource_mut::<bevy::state::state::NextState<RunningState>>()
            .set(RunningState::Options);
    }
    reached
}

/// Builds the headless walk app, injecting the persistent `Load` resources the
/// machine needs to traverse `Load` (no `AssetServer` under `MinimalPlugins`), plus
/// a `LoadedSituation` for the Generation setup to consume.
///
/// GTW-261 made the situation a gate-blocking `Load` resource, so a `LoadedSituation`
/// is ALWAYS seeded (symmetric with the theme/tuning/weapons seeds): the passed
/// `situation` fixture when `Some`, else the empty `Situation::default()`. The empty
/// default exercises the zero-ganger battle-build path the way the absent
/// `request_battle_setup` fallback used to.
pub(crate) fn walk_app(situation: Option<Situation>) -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // The Load-built WeaponRegistry (GTW-257): persistent `Load` state the real app
    // resolves from assets/content/weapons/ranged/, injected here for the MinimalPlugins deep-walk
    // (no AssetServer) so the Generation setup arms each ganger from it — the canonical
    // `test_weapon_registry` (GTW-324), which holds the `test-weapon` key every fixture
    // ganger references.
    app.world_mut().insert_resource(test_weapon_registry());
    app.world_mut()
        .insert_resource(test_melee_weapon_registry());
    // The Load-built ArmorRegistry (GTW-269) so the Generation setup armors each
    // ganger: every fixture ganger references the central `test-armor` key, which the
    // canonical `test_armor_registry` (GTW-324) holds (it must be populated now that
    // setup_battle resolves armor keys; the empty-default situation has zero gangers, so
    // even then this registry is harmless).
    app.world_mut().insert_resource(test_armor_registry());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry, AND the v2 setup_battle
    // resolves each fixture ganger's (gang, member) ref against it — so seed the canonical
    // `test_gang_registry` (which holds every `ganger_at` / default-builder member), NOT an
    // empty registry (an empty one would fail closed with GangNotFound and spawn nothing).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::test_support::test_gang_registry());
    // GTW-261: the Load→Intro gate now requires a LoadedSituation; seed the fixture
    // when given, else the empty default so the walk still traverses Load.
    app.world_mut()
        .insert_resource(LoadedSituation::new(situation.unwrap_or_default()));
    app
}

/// Drives the app from the default start down to `BattleScapeState::Generation`.
/// Returns whether Generation was reached within budget.
pub(crate) fn drive_to_generation(app: &mut bevy::app::App) -> bool {
    if !drive_past_menu(app) {
        return false;
    }
    advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::Generation),
        BUDGET,
    )
}
