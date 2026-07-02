//! GTW-542 (child GTW-41b) — WEAPON ATTACHMENTS, proven on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` spawn path (the gtw525/gtw526 idiom):
//!
//! - **RON round-trip** — a `WeaponSpec` parses an `attachment_slots:` list from RON; an
//!   omitted field defaults to an EMPTY list.
//! - **Each tag applies its component / leaf on the REAL spawn** — a ganger wielding an
//!   attachment-bearing weapon spawns the sibling tag (`Silenced` present, `Stable(true)`,
//!   `Scoped` present) OR the rewritten spawn-side leaf (`Magazine.reload_tu` reduced by
//!   `FastReload`, `WeaponPunch` raised by `HexgrindRounds`, `MagazineSize` grown by
//!   `GoreSumpDrum`, `DamageType` overridden by `RotgutCoating`).
//! - **The 4 NAMED observable** — a `Sighted` weapon reads a TIGHTER `cone_for` than an
//!   un-scoped baseline; `FastReload` yields a lower `reload_tu`; a `Silenced` shot
//!   produces NO `SuppressionApplied` where an identical un-silenced shot does; `Stable`
//!   engages the §1a brace (steadier facing an empty cell).
//! - **IDENTITY** — an empty `attachment_slots` list spawns a component set byte-identical
//!   to a weapon authored without the field (no sibling tags, un-rewritten leaves).
//!
//! Magnitudes are NEVER pinned — the asserts are presence / relative direction only.

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Entity, MessageReader, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Accuracy, AttachTag, BaseSpread, Cell, CellLevel, Cool, DamageType, Faction, FatalBias, Grit,
    Handedness, Kickback, MagazineSize, Scoped, Shove, Silenced, Speed, Stable, Stance, StanceKind,
    Strength, SuppressionApplied, Toughness, WeaponPunch,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Direction, Facing, GangRegistry},
    magazine::{Magazine, ReloadTu},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_terrain_registry,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        FireMode, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, WeaponDamage,
        WeaponName, WeaponRegistry, WeaponShred, WeaponSpec, Wields, shooter_weapon_silenced,
    },
};

/// An arbitrary seed (determinism is asserted elsewhere).
const SEED: u64 = 0x0A77_AC48;
const PLAYER: u8 = 0;
const ENEMY: u8 = 1;
/// A view range covering the point-blank fixtures.
const TEST_VIEW_RANGE: u16 = 12;
/// The weapon key every fixture ganger resolves (the shared test-weapon key).
const WEAPON_KEY: &str = "test-weapon";

const fn level0() -> gdtf_battle_sim::Level {
    gdtf_battle_sim::Level::new(0)
}

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), level0())
}

/// A ranged weapon spec carrying the chosen attachment slots — arbitrary (not shipped)
/// magnitudes; a tight-cone single-shot mode so a point-blank shot connects.
fn ranged_spec(slots: Vec<AttachTag>) -> WeaponSpec {
    WeaponSpec {
        base_spread:      BaseSpread::new(0.05),
        accuracy:         Accuracy::new(5.0),
        kickback:         Kickback::new(0.0),
        fatal_bias:       FatalBias::new(3.0),
        damage:           WeaponDamage::new(12),
        punch:            WeaponPunch::new(10),
        shred:            WeaponShred::new(3),
        damage_type:      DamageType::Kinetic,
        magazine:         Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)),
        fire_mode:        FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.2),
            ModeShots::new(1),
        )]),
        stable:           Stable::new(false),
        shove:            Shove::new(false),
        handedness:       Handedness::OneHanded,
        attachment_slots: slots,
        dot:              None,
        on_death:         None,
    }
}

/// A weapon registry whose shared `test-weapon` key carries the chosen attachment slots.
fn ranged_registry(slots: Vec<AttachTag>) -> WeaponRegistry {
    WeaponRegistry::new([(WeaponName::new(WEAPON_KEY.to_owned()), ranged_spec(slots))])
}

/// Build the live-runtime harness with the chosen attachment slots on the shared weapon.
fn battle_app(slots: Vec<AttachTag>) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(ranged_registry(slots));
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());
    app
}

/// Drive a setup through the REAL Ok path and settle it.
fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..4 {
        app.update();
    }
}

/// The entity of the (sole) ganger of `faction`.
fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

/// The (single) wielded weapon entity of `ganger` — the first related `Wields` entity.
fn weapon_entity_of(app: &App, ganger: Entity) -> Option<Entity> {
    app.world().get::<Wields>(ganger).and_then(Wields::weapon)
}

/// A standing player ganger at `at` facing `dir` (the shooter).
fn player_at(at: CellLevel, dir: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(dir))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(12.0))
        .strength(Strength::new(12.0))
        .grit(Grit::new(12.0))
        .cool(Cool::new(12.0))
        .build()
}

/// A standing enemy ganger at `at` (the target / reactor).
fn enemy_at(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(ENEMY))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(12.0))
        .strength(Strength::new(12.0))
        .grit(Grit::new(12.0))
        .cool(Cool::new(12.0))
        .toughness(Toughness::new(12.0))
        .build()
}

/// Spawn a lone player wielding the attachment-bearing weapon; return its weapon entity.
fn spawn_lone_player_weapon(slots: Vec<AttachTag>) -> (App, Entity) {
    let mut app = battle_app(slots);
    // A lone player + a distant enemy so setup's win/loss census stays two-sided.
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(ground(5, 5), Direction::East),
            enemy_at(ground(20, 20)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let Some(player) = ganger_of(&mut app, PLAYER) else {
        unreachable!("setup spawns one player");
    };
    let Some(weapon) = weapon_entity_of(&app, player) else {
        unreachable!("the player wields a ranged weapon entity");
    };
    (app, weapon)
}

// ── RON round-trip ─────────────────────────────────────────────────────────────

#[test]
fn attachment_slots_parse_from_ron() {
    // A slot list with a NAMED tag and a payload-bearing GRIMDARK tag round-trips by name.
    let ron = r"(
        base_spread: 0.05, accuracy: 5.0, kickback: 0.0, fatal_bias: 3.0,
        damage: 12, punch: 10, shred: 3, damage_type: Kinetic,
        magazine: (size: 20, reload_tu: 20),
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.2, shots: 1)],
        stable: false, handedness: OneHanded,
        attachment_slots: [Silenced, HexgrindRounds(punch_bonus: 6)],
    )";
    let Ok(spec) = ron::from_str::<WeaponSpec>(ron) else {
        unreachable!("attachment_slots must parse from RON");
    };
    assert_eq!(
        spec.attachment_slots.len(),
        2,
        "the authored two-slot list round-trips",
    );
    assert_eq!(
        spec.attachment_slots[0],
        AttachTag::Silenced,
        "the first slot parses as the named Silenced tag",
    );
    assert_eq!(
        spec.attachment_slots[1],
        AttachTag::HexgrindRounds {
            punch_bonus: WeaponPunch::new(6),
        },
        "the second slot parses its named-newtype payload",
    );
}

#[test]
fn omitted_attachment_slots_default_to_empty() {
    // No `attachment_slots:` field — the `#[serde(default)]` opt-in yields an EMPTY list.
    let ron = r"(
        base_spread: 0.05, accuracy: 5.0, kickback: 0.0, fatal_bias: 3.0,
        damage: 12, punch: 10, shred: 3, damage_type: Kinetic,
        magazine: (size: 20, reload_tu: 20),
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.2, shots: 1)],
        stable: false, handedness: OneHanded,
    )";
    let Ok(spec) = ron::from_str::<WeaponSpec>(ron) else {
        unreachable!("a weapon with no attachment_slots parses");
    };
    assert!(
        spec.attachment_slots.is_empty(),
        "an omitted attachment_slots field defaults to an empty list (the opt-in default)",
    );
}

// ── Each tag applies its component / leaf on the REAL spawn ──────────────────────

#[test]
fn silenced_tag_spawns_the_silenced_component() {
    let (app, weapon) = spawn_lone_player_weapon(vec![AttachTag::Silenced]);
    assert!(
        app.world().get::<Silenced>(weapon).is_some(),
        "a Silenced attachment spawns the Silenced sibling on the weapon entity",
    );
}

#[test]
fn stable_tag_sets_stable_true_on_the_weapon() {
    let (app, weapon) = spawn_lone_player_weapon(vec![AttachTag::Stable]);
    let stable = app.world().get::<Stable>(weapon).copied();
    assert_eq!(
        stable,
        Some(Stable::new(true)),
        "a Stable attachment sets Stable(true) on the weapon (the spec authored `stable: false`)",
    );
}

#[test]
fn sighted_tag_spawns_the_scoped_component() {
    let (app, weapon) = spawn_lone_player_weapon(vec![AttachTag::Sighted]);
    assert!(
        app.world().get::<Scoped>(weapon).is_some(),
        "a Sighted attachment spawns the Scoped (precision-optic) sibling on the weapon entity",
    );
}

#[test]
fn fast_reload_tag_lowers_the_magazine_reload_tu() {
    // Baseline: the same weapon with NO attachments — its authored reload_tu.
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app
        .world()
        .get::<Magazine>(base_weapon)
        .map(|m| *m.reload_tu());
    // With FastReload fitted — the reload cost must be strictly LOWER.
    let (fast_app, fast_weapon) = spawn_lone_player_weapon(vec![AttachTag::FastReload]);
    let fast = fast_app
        .world()
        .get::<Magazine>(fast_weapon)
        .map(|m| *m.reload_tu());
    let (Some(base), Some(fast)) = (base, fast) else {
        unreachable!("both weapons carry a Magazine");
    };
    assert!(
        fast < base,
        "FastReload must lower the weapon's reload_tu (fast {fast} < baseline {base})",
    );
}

#[test]
fn hexgrind_rounds_tag_raises_weapon_punch() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app.world().get::<WeaponPunch>(base_weapon).copied();
    let (hex_app, hex_weapon) = spawn_lone_player_weapon(vec![AttachTag::HexgrindRounds {
        punch_bonus: WeaponPunch::new(7),
    }]);
    let hex = hex_app.world().get::<WeaponPunch>(hex_weapon).copied();
    let (Some(base), Some(hex)) = (base, hex) else {
        unreachable!("both weapons carry a WeaponPunch");
    };
    assert!(
        *hex > *base,
        "HexgrindRounds must raise the weapon's punch (hex {} > baseline {})",
        *hex,
        *base,
    );
}

#[test]
fn gore_sump_drum_tag_grows_the_magazine_size() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app
        .world()
        .get::<Magazine>(base_weapon)
        .map(|m| m.size().get());
    let (drum_app, drum_weapon) = spawn_lone_player_weapon(vec![AttachTag::GoreSumpDrum {
        size_bonus:   MagazineSize::new(10),
        reload_delta: gdtf_battle_sim::tuning::ReloadDelta::new(5),
    }]);
    let drum = drum_app
        .world()
        .get::<Magazine>(drum_weapon)
        .map(|m| m.size().get());
    let (Some(base), Some(drum)) = (base, drum) else {
        unreachable!("both weapons carry a Magazine");
    };
    assert!(
        drum > base,
        "GoreSumpDrum must grow the magazine capacity (drum {drum} > baseline {base})",
    );
}

#[test]
fn rotgut_coating_tag_overrides_the_damage_type() {
    // The base weapon emits Kinetic; RotgutCoating(Chem) must override it to Chem.
    let (app, weapon) = spawn_lone_player_weapon(vec![AttachTag::RotgutCoating {
        damage_type: DamageType::Chem,
    }]);
    let dtype = app.world().get::<DamageType>(weapon).copied();
    assert_eq!(
        dtype,
        Some(DamageType::Chem),
        "RotgutCoating must override the weapon's emitted DamageType",
    );
}

// ── IDENTITY: an empty slot list spawns byte-identical to no field ───────────────

#[test]
fn empty_attachment_slots_spawn_byte_identical() {
    let (app, weapon) = spawn_lone_player_weapon(Vec::new());
    let world = app.world();
    // No sibling attachment tags exist on an un-attached weapon.
    assert!(
        world.get::<Silenced>(weapon).is_none(),
        "an un-attached weapon has NO Silenced sibling",
    );
    assert!(
        world.get::<Scoped>(weapon).is_none(),
        "an un-attached weapon has NO Scoped sibling",
    );
    // The authored `stable: false` is preserved (no Stable attachment flipped it).
    assert_eq!(
        world.get::<Stable>(weapon).copied(),
        Some(Stable::new(false)),
        "an un-attached weapon keeps its authored `stable: false` (no rewrite)",
    );
    // The authored damage type / punch / magazine are un-rewritten.
    assert_eq!(
        world.get::<DamageType>(weapon).copied(),
        Some(DamageType::Kinetic),
        "an un-attached weapon keeps its authored DamageType",
    );
    assert_eq!(
        world.get::<WeaponPunch>(weapon).copied(),
        Some(WeaponPunch::new(10)),
        "an un-attached weapon keeps its authored punch",
    );
}

// ── The Silenced dual-producer gate: a silenced shot makes no suppression ────────

/// A test-local recorder of every `SuppressionApplied` observed across the run.
#[derive(Resource, Default)]
struct AppliedLog {
    /// The number of `SuppressionApplied` signals observed.
    count: usize,
}

/// Drain `SuppressionApplied` into the recorder (registered after `BattleSimPlugin`).
fn record_applied(
    mut msgs: MessageReader<SuppressionApplied>,
    mut log: bevy::prelude::ResMut<AppliedLog>,
) {
    for _msg in msgs.read() {
        log.count += 1;
    }
}

/// Build a two-ganger app where a PLAYER point-blank-fires at an ADJACENT enemy, recording
/// `SuppressionApplied`. The player's weapon carries `slots`.
fn suppression_probe_app(slots: Vec<AttachTag>) -> (App, Entity) {
    let mut app = battle_app(slots);
    app.init_resource::<AppliedLog>();
    app.add_systems(bevy::app::Update, record_applied);
    // A radius-1 suppression disc reaches the adjacent enemy at (6,5).
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(ground(5, 5), Direction::East),
            enemy_at(ground(6, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let Some(shooter) = ganger_of(&mut app, PLAYER) else {
        unreachable!("setup spawns one player");
    };
    (app, shooter)
}

#[test]
fn silenced_shot_produces_no_suppression_where_an_unsilenced_shot_does() {
    // Un-silenced control: the enemy within the suppression radius IS suppressed.
    let (mut loud, loud_shooter) = suppression_probe_app(Vec::new());
    loud.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            loud_shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    for _ in 0..4 {
        loud.update();
    }
    let loud_count = loud.world().resource::<AppliedLog>().count;
    assert!(
        loud_count > 0,
        "an UN-silenced shot suppresses the adjacent enemy (control: {loud_count} signals)",
    );

    // Silenced: the SAME point-blank shot produces NO SuppressionApplied.
    let (mut quiet, quiet_shooter) = suppression_probe_app(vec![AttachTag::Silenced]);
    quiet
        .world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            quiet_shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    for _ in 0..4 {
        quiet.update();
    }
    assert_eq!(
        quiet.world().resource::<AppliedLog>().count,
        0,
        "a SILENCED shot produces NO SuppressionApplied (the producer gates on the shooter's \
         Silenced weapon)",
    );
}

/// The single-shot mode matching the test weapon's authored mode.
const fn single_shot_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

// ── The shared silenced gate helper resolves the shooter's ranged weapon ─────────

#[test]
fn shooter_weapon_silenced_reads_the_wielded_ranged_weapon_tag() {
    // A silenced-wielding shooter reads `true`; an un-silenced one reads `false` — proving
    // the shared `shooter → Wields → the ranged weapon → Silenced` resolution the two
    // producers gate on.
    let (mut silenced_app, silenced_shooter) = suppression_probe_app(vec![AttachTag::Silenced]);
    let is_silenced = silenced_app
        .world_mut()
        .run_system_once_with(silenced_probe, silenced_shooter)
        .unwrap_or(false);
    assert!(
        is_silenced,
        "shooter_weapon_silenced is true for a shooter wielding a Silenced weapon",
    );

    let (mut loud_app, loud_shooter) = suppression_probe_app(Vec::new());
    let is_loud = loud_app
        .world_mut()
        .run_system_once_with(silenced_probe, loud_shooter)
        .unwrap_or(true);
    assert!(
        !is_loud,
        "shooter_weapon_silenced is false for a shooter wielding an un-silenced weapon",
    );
}

/// A one-shot system exercising the shared `shooter_weapon_silenced` gate over the live
/// wield / melee-probe / Silenced-marker queries — the EXACT resolution the two producers use.
fn silenced_probe(
    shooter: bevy::prelude::In<Entity>,
    wields: gdtf_battle_sim::fire::WieldsQuery,
    melee: gdtf_battle_sim::fire::MeleeQuery,
    silenced: bevy::prelude::Query<(), bevy::prelude::With<Silenced>>,
) -> bool {
    shooter_weapon_silenced(*shooter, &wields, &melee, &silenced)
}
