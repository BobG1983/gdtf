//! The bootstrap drive itself: reaches Battlescape with the sim constructed,
//! panic-free + seed-deterministic, headless and stops at `BattleRunning`.

use bevy::state::state::State;
use gdtf_app::test_support::{BattleScapeState, GameState};
use gdtf_battle_sim::{
    acts::FireRequested,
    cover::{CoverLedger, HeightBand},
    ganger::{Hp, LifeState, Wounds},
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    rng::ShotRng,
    surface::SurfaceGrid,
    test_support::{key, single_mode},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
};

use super::harness::*;

/// The authored ganger count the fixture spawns — the AC1 `Wears`-count assertion
/// reads this exact number.
const AUTHORED_GANGER_COUNT: usize = 2;

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`GameState`] if it is active.
fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

/// AC1 — Bootstrap reaches `BattleScape` with the sim constructed. The shared
/// [`BattleAppBuilder`] (seeded with the persistent `Load` resources + the inline
/// [`bootstrap_situation`], started at [`AppState::Running`](gdtf_app::test_support::AppState::Running),
/// driven past the menu) descends to [`GameState::BattleScape`], and once
/// [`BattleScapeState::Generation`] has run E10.5's wired `setup_battle` the world holds
/// the four sim resources ([`OccupancyGrid`] / [`CoverLedger`] / [`SurfaceGrid`] /
/// [`VerticalLinkGraph`]), the seeded RNG streams, AND the [`CombatTuning`] present
/// through the battle — and `query::<&Wears>().count()` (the armor relationship every
/// ganger carries since GTW-323) equals the authored ganger count (the `setup_battle`
/// C8(a) entity-count precedent, proving the entities were spawned by the REAL setup,
/// not a no-op scaffold).
#[test]
fn bootstrap_reaches_battlescape_with_the_sim_constructed() {
    let app_opt = bootstrap_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should descend to BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // We descended through GameState::BattleScape (the Generation child ran on the way).
    assert_eq!(
        game_state(&app),
        Some(GameState::BattleScape),
        "the walk must rest inside GameState::BattleScape after reaching BattleRunning",
    );

    // The four sim resources E10.5's wired setup_battle inserts during Generation.
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "Generation's setup_battle must insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "Generation's setup_battle must insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "Generation's setup_battle must insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "Generation's setup_battle must insert a VerticalLinkGraph",
    );
    // The GTW-14 five-stream RNG: ShotRng is the first seeded during Generation.
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "Generation must insert the seeded RNG streams",
    );
    // CombatTuning is present through the battle (E10.4's persistent Load resource).
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning must be present in BattleRunning",
    );

    // The authored gangers were spawned as entities by the REAL setup_battle: exactly
    // one `Wears`-carrying ganger per authored ganger (count-equality, not a magnitude).
    // Since GTW-323 the armor stats live on related piece entities, so a ganger is
    // identified by its `Wears` armor relationship, not a `WornArmor` component.
    let world = app.world_mut();
    let mut worn = world.query::<&gdtf_battle_sim::armor::Wears>();
    assert_eq!(
        worn.iter(world).count(),
        AUTHORED_GANGER_COUNT,
        "the world must hold exactly the authored ganger count of gangers wearing armor (Wears) — \
         the real setup_battle spawned the gangers, not a no-op scaffold",
    );
}

/// AC4 — The boundary holds: no panic and seeded determinism. The whole drive
/// (Generation `setup_battle` + a `BattleRunning` `FireRequested` round) runs without
/// panic, and is reproducible: two independent bootstrap apps built from the SAME inline
/// [`bootstrap_situation`] and the SAME fixed [`BattleSeed`] (pre-injected by
/// [`bootstrap_app`] via [`BattleAppBuilder::with_seed`] so `request_battle_setup`
/// bypasses wall-clock entropy), driven through the identical sequence, produce the
/// identical observable outcome. The test builds-and-drives twice and asserts the
/// post-fire target `(Hp, Wounds, LifeState)` tuple is equal across the two runs; the
/// run completing the full descend+emit+assert sequence without aborting is the no-panic
/// evidence.
///
/// Both runs use [`BOOTSTRAP_SEED`] (via [`bootstrap_app`]): the test's concern is the
/// RELATION (same seed → same outcome), not any specific magnitude.
#[test]
fn the_drive_is_panic_free_and_seed_deterministic_across_runs() {
    // Run the full descend + arm + fire sequence once and snapshot the post-fire target
    // (Hp, Wounds, LifeState) tuple (all Copy + PartialEq). Returns None if the drive did
    // not reach BattleRunning or the setup did not spawn the gangers — the no-unwrap
    // let-else style so the test body stays panic-free.
    let post_fire_target_state = || -> Option<(Option<Hp>, Option<Wounds>, Option<LifeState>)> {
        // bootstrap_app pins BOOTSTRAP_SEED so both calls get the same BattleSeed
        // (bypassing wall-clock entropy in resolve_root_seed() for cross-run replay).
        let mut app = bootstrap_app()?;
        let shooter = find_ganger(&mut app, SHOOTER_FACTION)?;
        let target = find_ganger(&mut app, TARGET_FACTION)?;

        let mode = single_mode(0.2, 1);
        app.world_mut()
            .entity_mut(shooter)
            .insert(shooter_weapon_kit(mode));
        let (tx, ty, tl) = TARGET_AT;
        let target_at = key(tx, ty, tl);
        if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
            grid.set_occupant_band(target_at, Some(HeightBand::High));
        }

        app.world_mut().write_message(FireRequested::new(
            shooter,
            mode,
            Cell::new(tx, ty),
            Level::new(tl),
        ));
        app.update();

        Some((
            app.world().get::<Hp>(target).copied(),
            app.world().get::<Wounds>(target).copied(),
            app.world().get::<LifeState>(target).copied(),
        ))
    };

    let first = post_fire_target_state();
    let second = post_fire_target_state();
    assert!(
        first.is_some(),
        "the seeded drive must reach BattleRunning and fire",
    );
    assert_eq!(
        first, second,
        "the post-fire (Hp, Wounds, LifeState) tuple must be identical across two fixed-seed runs \
         (got {first:?} vs {second:?}) — the same BattleSeed source reproduces the same outcome",
    );
}

/// AC5 — Strictly headless, no out-of-slice edge, no path past `BattleRunning`. This test
/// re-encodes the headless contract behaviorally: the drive STOPS at
/// [`BattleScapeState::BattleRunning`] and the test never advances past it into
/// [`BattleScapeState::AfterMath`]. (The file's import set names no `Camera`, no
/// `Window`, no `gdtf_battle_presenter`, no `gdtf_battle_input`, no `*Resolved` type, and
/// no `AfterMath` variant — verified by inspection at gate, as `state_walk.rs`'s headless
/// contract is.)
#[test]
fn drive_is_headless_and_stops_at_battle_running() {
    let app_opt = bootstrap_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(app) = app_opt else {
        return;
    };

    // The drive rests AT BattleRunning — this slice's path stops here and does not enter
    // AfterMath (the AfterMath leg is intentionally out of this slice).
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "the headless drive must stop at BattleScapeState::BattleRunning, never advancing into \
         AfterMath",
    );
}
