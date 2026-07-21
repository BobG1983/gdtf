//! GTW-414 / GTW-415 / GTW-744 — the REAL-ASSET live-battle-start proof (the gangs
//! family's bespoke migration pin, split from `load_gangs.rs` under the repo file caps
//! by GTW-580 — kept verbatim, then repointed by GTW-744 to the deploy path).
//!
//! This is the feature-completeness proof the seeded-fixture regression tests cannot give:
//! it drives the WHOLE real menu→Load→battle flow — the genuine `AssetServer` rooted at the
//! workspace `assets/` loads `assets/content/gangs/*.gang.ron` into a `GangRegistry`, the real
//! prefab / theme / terrain registries, and the shipped `skirmish.ron` — WITHOUT seeding any
//! default registry, and proves the REAL shipped situation spawns the CONCRETE expected ganger
//! set through the authoritative live path.
//!
//! GTW-744 (procgen deployment): `skirmish.ron` now authors its combatants as `rosters` (gang +
//! member + faction refs, NO placement cells). So the placement is DERIVED by the procgen deploy
//! step (from the generated map's deployment zones), not authored — this test therefore drives
//! the FULL app to `BattleRunning` (the live `generate_level` → `deploy_rosters` → `setup_battle`
//! path) rather than calling `setup_battle` directly on the raw situation. What the migration
//! PRESERVES verbatim — the four ganger IDENTITIES, their eight attributes, their weapon / armor
//! keys, and their FACTION — is asserted field-for-field (the explicit pin-the-migrated-values
//! exception to the no-pinning rule); the SPAWN CELLS are procgen-derived, so the test asserts
//! only that all four deployed to distinct, live positions.
//!
//! It would FAIL closed if the GTW-415 loader were missing (the registry stays empty, every
//! roster member's `(gang, member)` ref fails to resolve → no gangers deploy/spawn → the battle
//! never reaches `BattleRunning`). The expected set + its field-for-field assertions live in
//! [`expected`].

/// The expected shipped spawn set + its field-for-field assertions (split under the repo
/// file caps). `#[path]` because a test-crate ROOT resolves a bare `mod` beside itself in
/// `tests/`, not in a same-named subdirectory.
#[path = "load_gangs_spawn/expected.rs"]
mod expected;

use std::collections::HashMap;

use bevy::{
    app::App,
    prelude::{Entity, NextState, World},
    state::state::State,
};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_battle_sim::ganger::{GangRegistry, GangerName};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// A generous budget for the real `DefaultPlugins` async asset loads + the full state descent
/// under contention (the `real_battle_panel.rs` / `procgen_battle.rs` precedent).
const BUDGET: u32 = 512;

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Part (b) — the REAL shipped `skirmish.ron` + the real `GangRegistry` (both resolved by the
/// genuine Load flow) drive the authoritative live path to spawn the CONCRETE expected ganger
/// set: same `GangerName`s (Alex Mercer / Kira Vann / Vex 1 / Vex 2), same eight attribute
/// values, same weapon, same armor key, and same faction as the pre-migration inline list.
///
/// GTW-744: the SPAWN CELLS are procgen-derived (the deploy step places each roster member into
/// its side's zone), so this asserts distinct live positions rather than authored cells; the
/// roster VALUES (identity + attributes + weapon/armor + faction) are the migration-preserved
/// data and are pinned field-for-field.
///
/// This is the live-battle-start-is-whole proof: it FAILS closed if the gang refs fail to
/// resolve (no gangers deploy → the battle never reaches `BattleRunning`).
#[test]
fn real_skirmish_with_real_gangs_spawns_the_expected_set() {
    // 1. Drive the REAL menu→Load→battle path (a live `AssetServer` rooted at the workspace
    //    `assets/`, no seeded defaults) to the running battle.
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    let reached_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(
        reached_menu,
        "the real Load flow must reach RunningState::Menu within {BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let reached_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_battle,
        "the real skirmish must generate + DEPLOY its roster and reach BattleRunning within \
         {BUDGET} updates (a missing gang ref would leave zero gangers and never reach it); last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // 2. Read the real, resolved-from-disk `GangRegistry` (the roster-key data proof resolves
    //    against it). It survives into the battle as a persistent Load resource.
    let gangs = app.world().get_resource::<GangRegistry>().cloned();
    assert!(
        gangs.is_some(),
        "the real Load flow must resolve a GangRegistry from the shipped gangs",
    );
    let Some(gangs) = gangs else {
        return;
    };

    // 3. Build a name → spawned-entity map in one query, then assert each expected ganger
    //    spawned with its concrete migrated values (attributes + weapon + faction), the migrated
    //    weapon + armor KEYS (data-level, via the real gang registry), and a distinct deployed
    //    position.
    let spawned = ganger_entities(app.world_mut());
    expected::assert_expected_set(app.world(), &spawned, &gangs);
}

/// Collect every spawned ganger entity keyed by its [`GangerName`] string — one query so the
/// per-ganger assertions read from a `&World` (no per-lookup `&mut World`).
fn ganger_entities(world: &mut World) -> HashMap<String, Entity> {
    let mut query = world.query::<(Entity, &GangerName)>();
    query
        .iter(world)
        .map(|(entity, name)| ((**name).clone(), entity))
        .collect()
}
