//! GTW-209 (E10.7): the headless end-to-end bootstrap CAPSTONE — a battle
//! constructs AND runs end-to-end under `MinimalPlugins`, with NO production code
//! of its own. It consumes ONLY what the prior E10 slices landed: the E10.1
//! `gdtf_app → gdtf_battle_sim` Cargo edge (so this crate can name sim types), the
//! E10.0 [`SimSystems::Simulate`] band, the E10.2 `*Requested` message contract +
//! per-act dispatch, the E10.5 `BattleSimPlugin` (Generation seeds the RNG streams
//! from the chosen [`BattleSeed`] source + inserts [`CombatTuning`] + runs
//! `setup_battle`), and the GTW-212 [`BattleInProgress`]-gated battle-wide dispatch
//! that E10.6 keeps live across `BattleRunning`.
//!
//! PROOF MIGRATION (GTW-324): this file is now built on the shared test
//! architecture — the drive-to-battle sequence is the crate-central
//! [`gdtf_test_utils::BattleAppBuilder`], and the authored battlefield + fixture
//! gangers come from the canonical [`gdtf_battle_sim::test_support`] builders
//! ([`GangerSpawnBuilder`] / [`SituationBuilder`]). It no longer hand-rolls a
//! `capstone_app`, its weapon/armor registries, or its `ganger_at` — those are the
//! shared builders the `BattleAppBuilder` seeds. The drive STOPS at
//! `BattleScapeState::BattleRunning` (never `AfterMath`).
//!
//! These are *pin-discriminating* tests: each `#[test]` re-encodes one acceptance
//! criterion as a before≠after / equality RELATION (never a pinned tunable magnitude),
//! so a regression in the E10 wiring turns the test red.
//!
//! NO function in this file takes `&mut World`/`&World`; every `app.world_mut()` /
//! `app.world()` call is in the TEST BODY (the established `gdtf_app` test idiom). No
//! `Camera`, no `Window`, no `gdtf_battle_presenter`, no `gdtf_battle_input`, no
//! `*Resolved` type, and the drive STOPS at `BattleScapeState::BattleRunning` (never
//! `AfterMath` / `AfterMathState`).

use bevy::{ecs::entity::Entity, state::state::State};
use gdtf_app::test_support::{BattleScapeState, GameState};
use gdtf_battle_sim::{
    acts::{FireRequested, SetStanceRequested},
    battle::BattleInProgress,
    cover::{CoverLedger, HeightBand},
    ganger::{Facing, Faction, Hp, LifeState, Stance, StanceKind, TuMax, Wounds},
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    rng::{BattleSeed, ShotRng},
    situation::Situation,
    surface::SurfaceGrid,
    test_support::{GangerSpawnBuilder, SituationBuilder, key},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};
use gdtf_test_utils::BattleAppBuilder;

/// The shooter's faction in the fixture (the ganger the drive proof arms + fires).
const SHOOTER_FACTION: u8 = 0;
/// The target's faction in the fixture (the ganger fired at).
const TARGET_FACTION: u8 = 1;

/// The `(cell, level)` the shooter is authored at — west of the target, same storey,
/// so a due-East shot reaches it.
const SHOOTER_AT: (i32, i32, u8) = (2, 5, 0);
/// The `(cell, level)` the target is authored at — due East of the shooter, close
/// range so the shot lands deterministically under the fixed seed.
const TARGET_AT: (i32, i32, u8) = (8, 5, 0);

/// The stance every fixture ganger is authored holding — the AC3 "authored start" value
/// the requested stance must differ from. Matches the
/// [`GangerSpawnBuilder`](gdtf_battle_sim::test_support::GangerSpawnBuilder) default.
const AUTHORED_STANCE: StanceKind = StanceKind::Standing;
/// The stance the AC3 `SetStanceRequested` asks for — DISTINCT from [`AUTHORED_STANCE`],
/// so a successful flip is observable as a change to exactly this value.
const REQUESTED_STANCE: StanceKind = StanceKind::Prone;

/// The authored ganger count the fixture spawns — the AC1 `Wears`-count assertion
/// reads this exact number.
const AUTHORED_GANGER_COUNT: usize = 2;

/// Build the bootstrap fixture's authored ganger at `at` / `faction` via the central
/// [`GangerSpawnBuilder`](gdtf_battle_sim::test_support::GangerSpawnBuilder): facing
/// East (so a due-East shot reaches the target), holding the [`AUTHORED_STANCE`],
/// referencing the central test weapon + armor keys (the default), so a setup arms +
/// armors it from the registries the [`BattleAppBuilder`] seeds.
fn bootstrap_ganger(at: CellLevel, faction: u8) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(gdtf_battle_sim::ganger::Direction::East))
        .stance(Stance::new(AUTHORED_STANCE))
        .build()
}

/// The bootstrap fixture: a shooter (faction [`SHOOTER_FACTION`]) facing East and a
/// target (faction [`TARGET_FACTION`]) directly East at close range, built over the
/// central [`SituationBuilder`](gdtf_battle_sim::test_support::SituationBuilder).
/// Link-free, so the setup validates trivially. The `SetupBattleRequested` the app
/// sends on `OnEnter(Generation)` pours this real battle into the world before
/// `BattleRunning`.
fn bootstrap_situation() -> Situation {
    let (sx, sy, sl) = SHOOTER_AT;
    let (tx, ty, tl) = TARGET_AT;
    SituationBuilder::new()
        .with_gangers([
            bootstrap_ganger(key(sx, sy, sl), SHOOTER_FACTION),
            bootstrap_ganger(key(tx, ty, tl), TARGET_FACTION),
        ])
        .build()
}

/// The fixed [`BattleSeed`] every bootstrap test run uses — the same as the
/// pre-GTW-14 `DEFAULT_BATTLE_SEED = BattleSeed::new(0)` constant, kept as `0`
/// so the seeded drive is guaranteed to land the shot at close range.
const BOOTSTRAP_SEED: BattleSeed = BattleSeed::new(0);

/// Build the bootstrap app already driven to a live battle via the shared
/// [`BattleAppBuilder`], seeded with the bootstrap [`bootstrap_situation`] and the
/// fixed [`BOOTSTRAP_SEED`] (guarantees the shot lands at close range and keeps
/// every test deterministic). Returns `None` if the shared drive does not reach
/// `BattleRunning` (the caller asserts).
fn bootstrap_app() -> Option<bevy::app::App> {
    BattleAppBuilder::new()
        .with_situation(bootstrap_situation())
        .with_seed(BOOTSTRAP_SEED)
        .build()
}

/// A single-shot fire-mode spec from arbitrary (non-pinned) per-mode numbers — the
/// `acts.rs` fire-test precedent.
const fn single_mode(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// The DETERMINISTIC weapon-state bundle the drive proof RE-ARMS the shooter with —
/// since GTW-257 `setup_battle` arms every ganger from the registry, this OVERWRITES
/// that registry weapon (via a second `insert` on the queried-from-`setup_battle`
/// shooter entity) with a precise, high-damage kit so the test's single shot lands in a
/// known regime. It ALSO supplies `TuMax`, which `setup_battle` does NOT add (the
/// `WeaponBundle` carries the `Magazine` grouping itself since GTW-275). Arbitrary
/// magnitudes (not shipped tuning). Mirrors the `acts.rs` weapon kit.
fn shooter_weapon_kit(mode: FireModeSpec) -> impl bevy::prelude::Bundle {
    let mag_size = MagazineSize::new(30);
    (
        WeaponBundle::new(
            WeaponName::new(String::from("test-weapon")),
            BaseSpread::new(0.05),
            Accuracy::new(2.0),
            Kickback::new(0.2),
            FatalBias::new(0.0),
            DamageProfile::new(
                WeaponDamage::new(40),
                WeaponPunch::new(20),
                WeaponShred::new(10),
                DamageType::Kinetic,
            ),
            // GTW-275: the WeaponBundle now carries the Magazine grouping, so the kit's
            // known 10-round load rides in the HandlingProfile (a separate Magazine in the
            // same bundle would be a duplicate-component panic).
            HandlingProfile::new(
                Magazine::new(10, mag_size, ReloadTu::new(12)),
                FireMode::new(vec![mode]),
                Stable::new(true),
            ),
        ),
        // The shooter query also reads TuMax — not authored by `setup_battle` — so the kit
        // supplies it (the Magazine is now part of the WeaponBundle above).
        TuMax::new(100),
    )
}

/// Find the spawned ganger entity of `faction` in the world (the real-path query the
/// drive proof uses instead of hand-spawning). Returns the FIRST match — the fixture
/// authors exactly one ganger per faction. Runs entirely off `app.world_mut()` (the
/// accepted test-body idiom): it spawns nothing and takes no `&mut World` helper param.
fn find_ganger(app: &mut bevy::app::App, faction: u8) -> Option<Entity> {
    let wanted = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find_map(|(entity, &f)| (f == wanted).then_some(entity))
}

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

/// AC2 — A `FireRequested` emitted inline in `BattleRunning` mutates the model. With the
/// shared builder rested in [`BattleScapeState::BattleRunning`], the drive proof QUERIES
/// the spawned shooter + target by [`Faction`] (off `app.world_mut()`, the test-body
/// idiom), ARMS the queried shooter via `entity_mut(..).insert(<weapon kit + TuMax +
/// Magazine>)` (a `GangerSpawn` authors no weapon), PUBLISHES the target's occupant band
/// in the live [`OccupancyGrid`] (the silhouette the band-free march reads to resolve a
/// `Ganger` hit), snapshots the target's `Hp`/`Wounds`/`LifeState`, emits a
/// [`FireRequested`] inline via the world message buffer, `update()`s once so the
/// GTW-212-gated battle-wide dispatch consumes it, and asserts ≥1 of
/// `Hp`/`Wounds`/`LifeState` changed — a landed hit, phrased as a before≠after relation,
/// never a pinned magnitude.
#[test]
fn fire_requested_in_battle_running_mutates_the_model() {
    let app_opt = bootstrap_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // The witness the GTW-212-gated battle-wide dispatch keys on is present in
    // BattleRunning, and the OccupancyGrid the march reads is too.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "the BattleInProgress witness must be present in BattleRunning (the dispatch gate)",
    );

    // Query the two SETUP-SPAWNED gangers (never hand-spawned): the shooter + the target.
    let shooter_found = find_ganger(&mut app, SHOOTER_FACTION);
    let target_found = find_ganger(&mut app, TARGET_FACTION);
    assert!(
        shooter_found.is_some(),
        "the real setup must have spawned a faction-{SHOOTER_FACTION} shooter ganger",
    );
    assert!(
        target_found.is_some(),
        "the real setup must have spawned a faction-{TARGET_FACTION} target ganger",
    );
    let (Some(shooter), Some(target)) = (shooter_found, target_found) else {
        return;
    };
    assert_ne!(shooter, target, "shooter and target are distinct entities");

    // ARM the queried shooter — insert the deterministic high-damage kit onto the
    // EXISTING setup-spawned entity (augment, never re-spawn), overwriting the registry
    // weapon so the test's single shot lands in a known regime.
    let mode = single_mode(0.2, 1);
    app.world_mut()
        .entity_mut(shooter)
        .insert(shooter_weapon_kit(mode));

    // PUBLISH the target's occupant band in the live grid — the band-free march reads it
    // to band the round vs the occupant. HIGH so a standing target is squarely in path.
    let (tx, ty, tl) = TARGET_AT;
    let target_at = key(tx, ty, tl);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

    // Snapshot the target's battle surfaces before the emit.
    let hp_before = app.world().get::<Hp>(target).copied();
    let wounds_before = app.world().get::<Wounds>(target).copied();
    let life_before = app.world().get::<LifeState>(target).copied();

    // Emit the fire request IN BattleRunning, then advance one update so the gated
    // battle-wide dispatch (live across the battle) consumes it and runs fire().
    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(tx, ty),
        Level::new(tl),
    ));
    app.update();

    let hp_after = app.world().get::<Hp>(target).copied();
    let wounds_after = app.world().get::<Wounds>(target).copied();
    let life_after = app.world().get::<LifeState>(target).copied();

    let target_changed =
        hp_after != hp_before || wounds_after != wounds_before || life_after != life_before;
    assert!(
        target_changed,
        "a FireRequested in BattleRunning must land a hit — a target component changed (hp \
         {hp_before:?}->{hp_after:?}, wounds {wounds_before:?}->{wounds_after:?}, life \
         {life_before:?}->{life_after:?})",
    );
}

/// AC3 — A `SetStanceRequested` flips its addressed component through the dispatch
/// boundary. Emitting a [`SetStanceRequested`] inline in `BattleRunning`, carrying the
/// actor [`Entity`] + a [`REQUESTED_STANCE`] DISTINCT from the [`AUTHORED_STANCE`] start,
/// then `update()`, mutates exactly that [`Stance`] on exactly that entity through the
/// GTW-212-gated, E10.2-owned dispatch + the landed `set_stance` verb. The test reads the
/// actor's [`Stance`] before and after, asserting it differs from the authored start
/// before and equals the requested value after — proving the message→dispatch→verb path
/// carries the actor [`Entity`] correctly (a relation, no tunable pinned).
#[test]
fn set_stance_requested_in_battle_running_flips_the_component() {
    let app_opt = bootstrap_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // Address the setup-spawned shooter as the posture actor (it carries Stance + Tu).
    let actor_found = find_ganger(&mut app, SHOOTER_FACTION);
    assert!(
        actor_found.is_some(),
        "the real setup must have spawned a faction-{SHOOTER_FACTION} shooter ganger",
    );
    let Some(actor) = actor_found else {
        return;
    };

    // The authored start stance is the distinct baseline the flip must move off of.
    let stance_before = app.world().get::<Stance>(actor).map(|s| **s);
    assert_eq!(
        stance_before,
        Some(AUTHORED_STANCE),
        "the spawned actor must hold the authored start stance before the request",
    );

    // Emit a stance request for a DISTINCT stance, then advance one update so the gated
    // dispatch consumes it and runs set_stance.
    app.world_mut()
        .write_message(SetStanceRequested::new(actor, REQUESTED_STANCE));
    app.update();

    let stance_after = app.world().get::<Stance>(actor).map(|s| **s);
    assert_eq!(
        stance_after,
        Some(REQUESTED_STANCE),
        "the SetStanceRequested must flip exactly the actor's Stance to the requested value \
         through the dispatch boundary (was {stance_before:?})",
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
