//! GTW-543 (child GTW-41c of GTW-41) — the LIVE weapon-emplacement act: a ganger ENTERS an
//! adjacent VACANT emplacement (a TU-costed context act), which mans the mount (state Occupied,
//! occupant band forced HIGH, the bolted-down mounted gun spawned + wielded), FIRES that mounted
//! gun through the real fire path (the ranged read PREFERS the `MountedWeapon` while occupied, the
//! `EmplacementStability` seam steadies it), then EXITS (a separate TU-costed act: band restored, the
//! mounted-weapon edge despawned, the occupant's own gun resolves again). Proven END-TO-END on the
//! REAL `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH the same
//! buffered `EnterEmplacementRequested` / `ExitEmplacementRequested` / `FireRequested` messages the
//! input seam writes.
//!
//! Clause contract:
//!
//! - **ENTER**: an 8-adjacent actor with enough TU mans a VACANT emplacement — state flips to
//!   Occupied, the actor's TU drops by the `enter_emplacement_tu` leaf, the occupant band is forced
//!   HIGH, and the mounted gun (a `MountedWeapon`-marked entity resolved from the emplacement's
//!   `MountedWeaponKey`) is spawned + wielded by the occupant.
//! - **FIRE**: while occupied, the occupant's fire resolves the MOUNTED weapon (the `ShotFired`
//!   carries the MOUNTED gun's `DamageType`, distinct from the ganger's own gun), through the real
//!   `dispatch_fire` → `fire()` path, seeded-deterministically (two same-seed runs agree).
//! - **EXIT**: the occupant dismounts (a separate TU-costed act) — state back to Vacant, the mount
//!   edge despawned (the occupant no longer wields a `MountedWeapon`), the band restored from
//!   stance, and TU dropped by the `exit_emplacement_tu` leaf.
//! - **Gate rejections**: an unaffordable-TU enter, a non-adjacent enter, and an already-occupied
//!   enter (no force-eject) each leave the state untouched and spend no TU.
//!
//! NO pinned tunable magnitudes: the tests assert TU-DROPPED-BY-THE-LEAF (read from the tuning
//! resource) / state-flipped / band-forced / weapon-resolved — never a specific TU number or
//! damage figure.
//!
//! HARNESS NOTE (the gtw508 idiom): the sim crate is the LOW crate, so it drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins` +
//! `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Accuracy, BaseSpread, Cool, DamageType, Faction, FatalBias, FireMode, FireModeSpec, Grit,
    Handedness, Kickback, Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    ReloadTu, Shove, Speed, Stable, Stance, StanceKind, Strength, Toughness, Tu, WeaponDamage,
    WeaponPunch, WeaponShred,
    acts::{EnterEmplacementRequested, ExitEmplacementRequested, FireRequested},
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Facing, GangRegistry},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    rng::BattleSeed,
    shot_fired::ShotFired,
    situation::{GangerSpawn, Situation},
    terrain::{
        emplacement::{
            EmplacementOccupant, EmplacementState, MountedWeaponEntity, MountedWeaponKey,
        },
        entity::TerrainCell,
    },
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{MountedWeapon, TrajectoryStyle, WeaponName, WeaponRegistry, WeaponSpec, WieldedBy},
};

/// Gang `0` is the player.
const PLAYER: u8 = 0;

/// The mounted-weapon registry key the test emplacement references.
const MOUNTED_KEY: &str = "test-mounted";

/// The ganger's OWN carried ranged weapon key (from `test_ranged_registry`) — its `DamageType` is
/// distinct from the mount's, so a `ShotFired`'s `DamageType` discriminates which gun fired.
const OWN_KEY: &str = "test-own-gun";

/// A view range covering the adjacent scene (arbitrary test tuning).
const TEST_VIEW_RANGE: u16 = 12;

/// The mounted gun's `DamageType` — DISTINCT from the ganger's own gun (below), so a `ShotFired`'s
/// `damage` field tells which weapon `dispatch_fire` resolved.
const MOUNT_DAMAGE_TYPE: DamageType = DamageType::Plasma;

/// The ganger's own carried gun's `DamageType` — distinct from the mount's.
const OWN_DAMAGE_TYPE: DamageType = DamageType::Las;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A single-shot `WeaponSpec` with the given `DamageType` (arbitrary, non-pinned handling numbers).
/// The two guns in the test differ ONLY in their `DamageType`, so a fired `ShotFired`'s `damage`
/// discriminates the mount from the ganger's own gun.
fn gun_spec(damage_type: DamageType) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.2),
        accuracy: Accuracy::new(1.0),
        kickback: Kickback::new(0.2),
        fatal_bias: FatalBias::new(0.0),
        damage: WeaponDamage::new(12),
        punch: WeaponPunch::new(5),
        shred: WeaponShred::new(3),
        damage_type,
        magazine: Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode: FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.3),
            ModeShots::new(1),
        )]),
        stable: Stable::new(false),
        shove: Shove::new(false),
        handedness: Handedness::OneHanded,
        trajectory: TrajectoryStyle::Straight,
        attachments: Vec::new(),
        dot: None,
        on_death: None,
    }
}

/// A `WeaponRegistry` holding BOTH the ganger's own carried gun ([`OWN_KEY`]) and the emplacement's
/// mounted gun ([`MOUNTED_KEY`]) — the two differ only in `DamageType`. Stands in for the app's
/// `Load`-built registry (the emplacement's `MountedWeaponKey` resolves `MOUNTED_KEY` against it).
fn test_ranged_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(OWN_KEY.to_owned()),
            gun_spec(OWN_DAMAGE_TYPE),
        ),
        (
            WeaponName::new(MOUNTED_KEY.to_owned()),
            gun_spec(MOUNT_DAMAGE_TYPE),
        ),
    ])
}

/// Build the FULL live-runtime harness (the gtw508 `battle_app` idiom): `MinimalPlugins` +
/// `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin`, a `CombatTuning` (view range + default
/// emplacement TU leaves), and the persistent `Load` registries (with the two-gun ranged registry).
fn battle_app(seed: u64) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_ranged_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path with `seed` and settle it.
fn drive_setup(app: &mut App, seed: u64, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(seed)));
    for _ in 0..4 {
        app.update();
    }
}

/// The entity of the (sole) player ganger.
fn player_ganger(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == PLAYER)
        .map(|(entity, _)| entity)
}

/// A full-stat player ganger carrying the OWN-gun key so its own ranged weapon resolves at setup.
fn player_at(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .weapon(WeaponName::new(OWN_KEY.to_owned()))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

/// Spawn a VACANT emplacement terrain entity at `at` carrying the enter/exit state components +
/// the mounted-weapon key + a seeded (armorless) `CoverLedger` entry — exactly what `setup_battle`
/// attaches for an `Emplacement` piece. Returns its `Entity`.
fn spawn_emplacement(app: &mut App, at: CellLevel) -> Entity {
    let entity = app
        .world_mut()
        .spawn((
            TerrainCell::new(at),
            EmplacementState::Vacant,
            MountedWeaponKey::new(WeaponName::new(MOUNTED_KEY.to_owned())),
        ))
        .id();
    app.world_mut()
        .resource_mut::<CoverLedger>()
        .insert(at, cover_entry());
    entity
}

/// An intact armorless cover entry (the emplacement's structural HP; armor irrelevant to this test).
const fn cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(45),
        HeightBand::High,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    )
}

/// The emplacement's current [`EmplacementState`].
fn state(app: &App, entity: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(entity).copied()
}

/// The emplacement's recorded occupant, if any.
fn occupant(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<EmplacementOccupant>(entity).map(|o| **o)
}

/// The emplacement's recorded mounted-weapon entity, if any.
fn mount_entity(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<MountedWeaponEntity>(entity).map(|m| **m)
}

/// The grid's occupant band at `at`.
fn occupant_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&at))
}

/// The current TU of `entity`.
fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

/// The shipped `enter_emplacement_tu` leaf (read from the tuning resource, never hard-coded).
fn enter_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.enter_emplacement_tu)
}

/// The shipped `exit_emplacement_tu` leaf (read from the tuning resource, never hard-coded).
fn exit_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.exit_emplacement_tu)
}

/// Whether `ganger` currently wields a [`MountedWeapon`]-marked entity — resolved by scanning the
/// [`MountedWeapon`]-marked weapon entities for one whose [`WieldedBy`] back-reference points at
/// `ganger`. `true` only while manning an emplacement (the mount is spawned + related on enter,
/// despawned on exit).
fn wields_mount(app: &mut App, ganger: Entity) -> bool {
    use bevy::ecs::relationship::Relationship;
    let world = app.world_mut();
    let mut query = world.query_filtered::<&WieldedBy, bevy::prelude::With<MountedWeapon>>();
    query
        .iter(world)
        .any(|wielded_by| wielded_by.get() == ganger)
}

/// Step `app` a fixed number of ticks so a written message dispatches + settles.
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// Every [`ShotFired`] observed across the run (a `MessageReader` only sees the current+previous
/// update, so a recorder resource accumulates them).
#[derive(Resource, Default)]
struct ShotLog {
    /// One entry per `ShotFired` emitted.
    shots: Vec<ShotFired>,
}

/// Drain `ShotFired` into the recorder.
fn record_shots(
    mut fired: bevy::prelude::MessageReader<ShotFired>,
    mut log: bevy::prelude::ResMut<ShotLog>,
) {
    for shot in fired.read() {
        log.shots.push(shot.clone());
    }
}

/// Add the `ShotFired` recorder (after `BattleSimPlugin`, so the buffer exists).
fn with_shot_log(app: &mut App) {
    app.init_resource::<ShotLog>();
    app.add_systems(bevy::app::Update, record_shots);
}

/// The `DamageType` of the FIRST recorded `ShotFired`, if any (discriminates which gun fired).
fn first_shot_damage_type(app: &App) -> Option<DamageType> {
    app.world()
        .get_resource::<ShotLog>()
        .and_then(|log| log.shots.first().map(|s| s.damage))
}

// ── ENTER → (mount wielded) → EXIT (mount despawned) ────────────────────────────

/// The full enter/exit lifecycle: entering mans the emplacement (Occupied, TU spent, band HIGH,
/// mount spawned + recorded + wielded); exiting reverts everything (Vacant, band restored, mount
/// despawned + record cleared, TU spent).
#[test]
fn enter_mans_and_spawns_mount_exit_reverts_and_despawns_mount() {
    let (mut app, seed) = battle_app(0x5543_0A0A);
    // Player at (5,5); the emplacement is directly east at (6,5), 8-adjacent.
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let emp_cell = ground(6, 5);
    let emplacement = spawn_emplacement(&mut app, emp_cell);
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    let (Some(tu_before), Some(enter_cost)) =
        (tu_of(&app, actor), Some(enter_tu(&app)).filter(|c| *c > 0))
    else {
        unreachable!("the actor carries Tu and the enter leaf is a real positive cost");
    };
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "the emplacement starts Vacant",
    );

    // ENTER.
    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "ENTER: the emplacement is Occupied",
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(actor),
        "ENTER: the actor is recorded as the occupant",
    );
    assert_eq!(
        tu_of(&app, actor),
        Some(tu_before - enter_cost),
        "ENTER: the actor's TU dropped by EXACTLY the enter_emplacement_tu leaf",
    );
    assert_eq!(
        occupant_band(&app, emp_cell),
        Some(HeightBand::High),
        "ENTER: the occupant band is forced HIGH (reads as HIGH cover)",
    );
    assert!(
        mount_entity(&app, emplacement).is_some(),
        "ENTER: the emplacement records the spawned mounted-weapon entity",
    );
    assert!(
        wields_mount(&mut app, actor),
        "ENTER: the occupant wields the MountedWeapon (spawned + related on enter)",
    );

    let tu_after_enter = tu_of(&app, actor).unwrap_or(0);
    let exit_cost = exit_tu(&app);
    assert!(exit_cost > 0, "the exit leaf is a real positive cost");

    // EXIT.
    app.world_mut()
        .write_message(ExitEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "EXIT: the emplacement is Vacant again",
    );
    assert_eq!(
        occupant(&app, emplacement),
        None,
        "EXIT: the occupant record is cleared",
    );
    assert_eq!(
        mount_entity(&app, emplacement),
        None,
        "EXIT: the mounted-weapon record is cleared",
    );
    assert!(
        !wields_mount(&mut app, actor),
        "EXIT: the occupant no longer wields a MountedWeapon (the mount edge was despawned)",
    );
    assert_eq!(
        occupant_band(&app, emp_cell),
        Some(HeightBand::High),
        "EXIT: the band is restored from the STANDING occupant's stance silhouette (HIGH)",
    );
    assert_eq!(
        tu_of(&app, actor),
        Some(tu_after_enter - exit_cost),
        "EXIT: the actor's TU dropped by EXACTLY the exit_emplacement_tu leaf",
    );
}

// ── FIRE: the occupied ganger fires the MOUNTED weapon (not its own gun) ────────

/// An enemy ganger at `at` facing `facing` (gang 1) — a target for the mounted-gun fire test.
fn enemy_at(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(1))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .weapon(WeaponName::new(OWN_KEY.to_owned()))
        .toughness(Toughness::new(12.0))
        .build()
}

/// While OCCUPIED, the ganger's fire resolves the MOUNTED weapon: a `FireRequested` produces a
/// `ShotFired` carrying the MOUNTED gun's `DamageType` (distinct from the ganger's own gun), through
/// the real `dispatch_fire` → `fire()` path, and the outcome is seed-DETERMINISTIC (two same-seed
/// runs agree).
#[test]
fn occupied_ganger_fires_the_mounted_weapon_deterministically() {
    // Run the enter→fire flow and return the FIRST ShotFired's DamageType (the discriminator).
    let run = |seed: u64| -> Option<DamageType> {
        let (mut app, seed) = battle_app(seed);
        with_shot_log(&mut app);
        // Player at (5,5) facing East; emplacement at (6,5); enemy further East at (7,5) — in arc.
        let situation = SituationBuilder::new()
            .with_gangers([
                player_at(ground(5, 5), Direction::East),
                enemy_at(ground(7, 5), Direction::West),
            ])
            .build_with_gangs();
        drive_setup(&mut app, seed, situation);
        let emplacement = spawn_emplacement(&mut app, ground(6, 5));
        // The player is the gang-0 ganger at (5,5).
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &Faction, &gdtf_battle_sim::ganger::Position)>();
        let actor = q
            .iter(world)
            .find(|(_, f, p)| ***f == PLAYER && ***p == ground(5, 5))
            .map(|(e, ..)| e);
        let Some(actor) = actor else {
            unreachable!("the player ganger spawned at (5,5)");
        };

        // ENTER — man the emplacement (spawns + wields the mount).
        app.world_mut()
            .write_message(EnterEmplacementRequested::new(actor, emplacement));
        step(&mut app, 3);
        assert!(
            wields_mount(&mut app, actor),
            "precondition: the occupant wields the mount before firing",
        );

        // FIRE at the enemy cell — the ranged read PREFERS the mount while occupied.
        let mode = FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.3),
            ModeShots::new(1),
        );
        app.world_mut().write_message(FireRequested::new(
            actor,
            mode,
            Cell::new(7, 5),
            Level::new(0),
        ));
        step(&mut app, 3);
        first_shot_damage_type(&app)
    };

    let a = run(0x5543_0E0E);
    let b = run(0x5543_0F0F);
    assert_eq!(
        a,
        Some(MOUNT_DAMAGE_TYPE),
        "FIRE: the occupied ganger's shot carries the MOUNTED gun's DamageType (the mount was \
         resolved, not the ganger's own {OWN_DAMAGE_TYPE:?} gun)",
    );
    assert_eq!(
        a, b,
        "FIRE: the mounted-gun shot is seed-independent in WHICH weapon fires (both seeds resolve \
         the mount) — determinism of the resolution",
    );
}

// ── Gate rejections: unaffordable / non-adjacent / already-occupied ─────────────

/// An actor that cannot afford the enter leaf is rejected — the emplacement stays Vacant and its
/// (too-small) TU is untouched.
#[test]
fn unaffordable_enter_is_rejected_no_charge() {
    let (mut app, seed) = battle_app(0x5543_0B0B);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = spawn_emplacement(&mut app, ground(6, 5));
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    // Drain the actor's TU below the enter cost.
    let broke = enter_tu(&app).saturating_sub(1);
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(actor) {
        *tu = Tu::new(broke);
    }
    let tu_before = tu_of(&app, actor);

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "an unaffordable enter does not man the emplacement (the afford gate held)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "a rejected (unaffordable) enter spends NO TU",
    );
    assert!(
        !wields_mount(&mut app, actor),
        "a rejected enter spawns no mount",
    );
}

/// A non-adjacent actor is rejected — the emplacement stays Vacant and no TU is spent.
#[test]
fn non_adjacent_enter_is_rejected_no_charge() {
    let (mut app, seed) = battle_app(0x5543_0C0C);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    // The emplacement is THREE cells east — outside the 8-adjacent reach.
    let emplacement = spawn_emplacement(&mut app, ground(8, 5));
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    let tu_before = tu_of(&app, actor);

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "a non-adjacent enter does not man the emplacement (the 8-adjacency gate held)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "a rejected (non-adjacent) enter spends NO TU",
    );
}

/// Re-entering an ALREADY-occupied emplacement with a second ganger is a no-op (no force-eject) —
/// the first occupant stays seated and the second spends no TU.
#[test]
fn enter_on_occupied_is_rejected_no_force_eject() {
    let (mut app, seed) = battle_app(0x5543_0D0D);
    // Two adjacent players: first at (5,5), second at (7,5), emplacement between them at (6,5).
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(ground(5, 5), Direction::East),
            player_at(ground(7, 5), Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emp_cell = ground(6, 5);
    let emplacement = spawn_emplacement(&mut app, emp_cell);

    // Resolve the two player entities by their cells.
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Faction, &gdtf_battle_sim::ganger::Position)>();
    let players: Vec<(Entity, CellLevel)> = q
        .iter(world)
        .filter(|(_, f, _)| ***f == PLAYER)
        .map(|(e, _, p)| (e, **p))
        .collect();
    let (Some(first), Some(second)) = (
        players
            .iter()
            .find(|(_, p)| *p == ground(5, 5))
            .map(|(e, _)| *e),
        players
            .iter()
            .find(|(_, p)| *p == ground(7, 5))
            .map(|(e, _)| *e),
    ) else {
        unreachable!("both players spawned at their authored cells");
    };

    // The FIRST mans the emplacement.
    app.world_mut()
        .write_message(EnterEmplacementRequested::new(first, emplacement));
    step(&mut app, 3);
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "the first ganger mans the emplacement",
    );
    let second_tu_before = tu_of(&app, second);

    // The SECOND tries to enter the already-occupied emplacement.
    app.world_mut()
        .write_message(EnterEmplacementRequested::new(second, emplacement));
    step(&mut app, 3);
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "re-entering an occupied emplacement does NOT displace the seated occupant (no force-eject)",
    );
    assert_eq!(
        tu_of(&app, second),
        second_tu_before,
        "a rejected (already-occupied) enter spends NO TU for the second ganger",
    );
}
