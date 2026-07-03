//! GTW-549 (child GTW-551) — DATA-DRIVEN WEAPON ATTACHMENTS, proven on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` spawn + `apply_pending_attachments` path
//! (SUPERSEDES the GTW-542 `AttachTag`-enum model):
//!
//! - **RON round-trip** — an `AttachmentSpec` parses an `effects:` list from RON (each effect
//!   keyed by variant name with its per-item magnitude); a weapon references items BY KEY in
//!   its `attachments:` list (an omitted field defaults to an EMPTY list).
//! - **Each effect applies to the CORRECT stat on the REAL spawn** — a ganger wielding an
//!   attachment-bearing weapon has, after the post-spawn application: `Silenced` present
//!   (`Silence`), `Accuracy` RAISED (`Aim` → the HEADLINE fix, a sight boosts AIM not
//!   stability), `WeaponBraceBonus` present (`Stability` → the brace seam), `Magazine.size`
//!   grown (`ExtraAmmo`), `Magazine.reload_tu` lowered (`ReloadTime`).
//! - **The Silenced dual-producer gate** — a `Silence` attachment yields NO `SuppressionApplied`
//!   where an identical un-silenced shot does; the shared `shooter_weapon_silenced` gate reads
//!   the wielded ranged weapon's tag.
//! - **IDENTITY** — an empty `attachments` list spawns a weapon with no attachment effects
//!   (no `Silenced`, no `WeaponBraceBonus`, un-rewritten stats).
//! - **Loader tests do NOT pin shipped magnitudes** — every assert checks presence / relative
//!   direction against a distinctive inline baseline, never a shipped number.

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Entity, MessageReader, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Accuracy, BaseSpread, Cell, CellLevel, Cool, DamageType, Faction, FatalBias, Grit, Handedness,
    Kickback, MagazineSize, Shove, Silenced, Speed, Stable, Stance, StanceKind, Strength,
    SuppressionApplied, Toughness, WeaponPunch,
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
        AimDelta, AttachmentEffect, AttachmentName, AttachmentRegistry, AttachmentSlot,
        AttachmentSpec, FireMode, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        ReloadTimeScale, SlotCapacity, TrajectoryStyle, WeaponBraceBonus, WeaponDamage, WeaponName,
        WeaponRegistry, WeaponShred, WeaponSlots, WeaponSpec, shooter_weapon_silenced,
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
/// The attachment item key the fixture weapon references.
const ATTACHMENT_KEY: &str = "test-attachment";

const fn level0() -> gdtf_battle_sim::Level {
    gdtf_battle_sim::Level::new(0)
}

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), level0())
}

/// A ranged weapon spec referencing the chosen attachment KEYS — arbitrary (not shipped)
/// magnitudes; a tight-cone single-shot mode so a point-blank shot connects.
fn ranged_spec(attachment_keys: Vec<AttachmentName>) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.05),
        accuracy:    Accuracy::new(5.0),
        kickback:    Kickback::new(0.0),
        fatal_bias:  FatalBias::new(3.0),
        damage:      WeaponDamage::new(12),
        punch:       WeaponPunch::new(10),
        shred:       WeaponShred::new(3),
        damage_type: DamageType::Kinetic,
        magazine:    Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)),
        fire_mode:   FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.2),
            ModeShots::new(1),
        )]),
        stable:      Stable::new(false),
        shove:       Shove::new(false),
        handedness:  Handedness::OneHanded,
        trajectory:  TrajectoryStyle::Straight,
        // GTW-554: declare the Rail slot the shared fixture item occupies, so the
        // referenced key still FITS under the slot gate (capacity 1 — one fixture item).
        slots:       WeaponSlots::new(vec![(AttachmentSlot::Rail, SlotCapacity::new(1))]),
        attachments: attachment_keys,
        dot:         None,
        on_death:    None,
    }
}

/// A weapon registry whose shared `test-weapon` key references the chosen attachment keys.
fn ranged_registry(attachment_keys: Vec<AttachmentName>) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(WEAPON_KEY.to_owned()),
        ranged_spec(attachment_keys),
    )])
}

/// An attachment registry with the shared `test-attachment` key carrying `effects`.
fn attachment_registry(effects: Vec<AttachmentEffect>) -> AttachmentRegistry {
    AttachmentRegistry::new([(
        AttachmentName::new(ATTACHMENT_KEY.to_owned()),
        AttachmentSpec {
            display_name: WeaponName::new("Test Attachment".to_owned()),
            // GTW-554: the fixture item occupies the Rail slot the fixture weapon declares.
            slot: AttachmentSlot::Rail,
            effects,
        },
    )])
}

/// Build the live-runtime harness. The shared weapon references the `test-attachment` key iff
/// `effects` is non-empty; the attachment registry carries `effects` under that key.
fn battle_app(effects: Vec<AttachmentEffect>) -> App {
    let keys = if effects.is_empty() {
        Vec::new()
    } else {
        vec![AttachmentName::new(ATTACHMENT_KEY.to_owned())]
    };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(ranged_registry(keys));
    app.insert_resource(attachment_registry(effects));
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());
    app
}

/// Drive a setup through the REAL Ok path and settle it — enough updates for the weapon scene
/// to materialize AND the post-spawn `apply_pending_attachments` system to apply its effects.
fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    // Setup(Update) → weapon scene(SpawnScene) → apply_pending_attachments(next Update) →
    // its queued attach_to_weapon commands flush + the effect's own reinsert settles: a
    // generous settle window covers the multi-tick deferred cascade.
    for _ in 0..8 {
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

/// The (single) wielded RANGED weapon entity of `ganger` — the wielded entity carrying an
/// `Accuracy` stat (the melee weapon / fists does not), scanned over the world's weapon
/// entities related to `ganger` via `WieldedBy`.
fn weapon_entity_of(app: &mut App, ganger: Entity) -> Option<Entity> {
    use bevy::ecs::relationship::Relationship;
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &gdtf_battle_sim::weapon::WieldedBy, &Accuracy)>();
    query
        .iter(world)
        .find(|(_, wielded, _)| wielded.get() == ganger)
        .map(|(entity, ..)| entity)
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

/// Spawn a lone player wielding the attachment-bearing weapon; return its weapon entity (after
/// the post-spawn application has settled).
fn spawn_lone_player_weapon(effects: Vec<AttachmentEffect>) -> (App, Entity) {
    let mut app = battle_app(effects);
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
    let Some(weapon) = weapon_entity_of(&mut app, player) else {
        unreachable!("the player wields a ranged weapon entity");
    };
    (app, weapon)
}

// ── RON round-trip ─────────────────────────────────────────────────────────────

#[test]
fn attachment_spec_parses_effects_from_ron() {
    // An attachment item's `effects:` list round-trips by variant name, each with its payload.
    let ron = r#"(
        display_name: "Bionic Sight",
        slot: Sight,
        effects: [ Aim(0.4), Silence, Penetration(6) ],
    )"#;
    let Ok(spec) = ron::from_str::<AttachmentSpec>(ron) else {
        unreachable!("an AttachmentSpec must parse its effects list from RON");
    };
    assert_eq!(
        spec.slot,
        AttachmentSlot::Sight,
        "the GTW-554 slot the item occupies parses from RON",
    );
    assert_eq!(
        spec.effects.len(),
        3,
        "the authored three-effect list round-trips"
    );
    assert_eq!(
        spec.effects[0],
        AttachmentEffect::Aim(AimDelta::new(0.4)),
        "the first effect parses as Aim with its per-item AimDelta payload",
    );
    assert_eq!(
        spec.effects[1],
        AttachmentEffect::Silence,
        "the second effect parses as the no-payload Silence variant",
    );
}

#[test]
fn weapon_attachments_and_omitted_field_parse_from_ron() {
    // A weapon references attachment items BY KEY in its `attachments:` list.
    let with = r#"(
        base_spread: 0.05, accuracy: 5.0, kickback: 0.0, fatal_bias: 3.0,
        damage: 12, punch: 10, shred: 3, damage_type: Kinetic,
        magazine: (size: 20, reload_tu: 20),
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.2, shots: 1)],
        stable: false, handedness: OneHanded,
        slots: [(Sight, 1), (Muzzle, 1)],
        attachments: ["bionic_sight", "suppressor"],
    )"#;
    let Ok(spec) = ron::from_str::<WeaponSpec>(with) else {
        unreachable!("a weapon's attachments key list must parse from RON");
    };
    assert_eq!(
        spec.attachments.len(),
        2,
        "the authored two-key list round-trips"
    );
    assert_eq!(
        spec.slots.capacity(AttachmentSlot::Sight),
        Some(SlotCapacity::new(1)),
        "the GTW-554 slots pair-list parses from the weapon RON",
    );
    assert_eq!(
        spec.attachments[0],
        AttachmentName::new("bionic_sight".to_owned()),
        "the first key parses as a bare RON string",
    );

    // No `attachments:` field — the `#[serde(default)]` opt-in yields an EMPTY list.
    let without = r"(
        base_spread: 0.05, accuracy: 5.0, kickback: 0.0, fatal_bias: 3.0,
        damage: 12, punch: 10, shred: 3, damage_type: Kinetic,
        magazine: (size: 20, reload_tu: 20),
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.2, shots: 1)],
        stable: false, handedness: OneHanded,
    )";
    let Ok(spec) = ron::from_str::<WeaponSpec>(without) else {
        unreachable!("a weapon with no attachments field parses");
    };
    assert!(
        spec.attachments.is_empty(),
        "an omitted attachments field defaults to an empty list (the opt-in default)",
    );
}

// ── Each effect applies to the CORRECT stat on the REAL spawn ────────────────────

#[test]
fn silence_effect_spawns_the_silenced_component() {
    let (app, weapon) = spawn_lone_player_weapon(vec![AttachmentEffect::Silence]);
    assert!(
        app.world().get::<Silenced>(weapon).is_some(),
        "a Silence attachment fits the Silenced tag on the weapon entity (post-spawn apply)",
    );
}

#[test]
fn aim_effect_raises_accuracy() {
    // The HEADLINE fix: a sight boosts AIM (Accuracy), not stability. Baseline authored 5.0.
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app.world().get::<Accuracy>(base_weapon).map(|a| **a);
    let (aim_app, aim_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::Aim(AimDelta::new(0.5))]);
    let aimed = aim_app.world().get::<Accuracy>(aim_weapon).map(|a| **a);
    let (Some(base), Some(aimed)) = (base, aimed) else {
        unreachable!("both weapons carry Accuracy");
    };
    assert!(
        aimed > base,
        "an Aim attachment raises the weapon's Accuracy (aimed {aimed} > baseline {base})",
    );
}

#[test]
fn stability_effect_inserts_a_weapon_brace_bonus() {
    // Stability maps to the brace seam — a graduated WeaponBraceBonus component.
    let (app, weapon) = spawn_lone_player_weapon(vec![AttachmentEffect::Stability(
        WeaponBraceBonus::new(12.0),
    )]);
    assert!(
        app.world().get::<WeaponBraceBonus>(weapon).is_some(),
        "a Stability attachment inserts a WeaponBraceBonus (the §1a brace seam) on the weapon",
    );
    // The un-braced baseline carries NO such component.
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    assert!(
        base_app
            .world()
            .get::<WeaponBraceBonus>(base_weapon)
            .is_none(),
        "an un-braced weapon carries no WeaponBraceBonus",
    );
}

#[test]
fn extra_ammo_effect_grows_the_magazine_size() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app
        .world()
        .get::<Magazine>(base_weapon)
        .map(|m| m.size().get());
    let (drum_app, drum_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::ExtraAmmo(MagazineSize::new(10))]);
    let drum = drum_app
        .world()
        .get::<Magazine>(drum_weapon)
        .map(|m| m.size().get());
    let (Some(base), Some(drum)) = (base, drum) else {
        unreachable!("both weapons carry a Magazine");
    };
    assert!(
        drum > base,
        "an ExtraAmmo attachment grows the magazine capacity (drum {drum} > baseline {base})",
    );
}

#[test]
fn reload_time_effect_lowers_the_magazine_reload_tu() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app
        .world()
        .get::<Magazine>(base_weapon)
        .map(|m| *m.reload_tu());
    let (fast_app, fast_weapon) = spawn_lone_player_weapon(vec![AttachmentEffect::ReloadTime(
        ReloadTimeScale::new(0.5),
    )]);
    let fast = fast_app
        .world()
        .get::<Magazine>(fast_weapon)
        .map(|m| *m.reload_tu());
    let (Some(base), Some(fast)) = (base, fast) else {
        unreachable!("both weapons carry a Magazine");
    };
    assert!(
        fast < base,
        "a ReloadTime attachment lowers the weapon's reload_tu (fast {fast} < baseline {base})",
    );
}

// ── IDENTITY: an empty attachments list spawns with no attachment effects ─────────

#[test]
fn empty_attachments_spawn_with_no_effects() {
    let (app, weapon) = spawn_lone_player_weapon(Vec::new());
    let world = app.world();
    assert!(
        world.get::<Silenced>(weapon).is_none(),
        "an un-attached weapon has NO Silenced sibling",
    );
    assert!(
        world.get::<WeaponBraceBonus>(weapon).is_none(),
        "an un-attached weapon has NO WeaponBraceBonus",
    );
    // The authored `stable: false` + DamageType + Accuracy are un-rewritten.
    assert_eq!(
        world.get::<Stable>(weapon).copied(),
        Some(Stable::new(false)),
        "an un-attached weapon keeps its authored `stable: false`",
    );
    assert_eq!(
        world.get::<DamageType>(weapon).copied(),
        Some(DamageType::Kinetic),
        "an un-attached weapon keeps its authored DamageType",
    );
    assert_eq!(
        world.get::<Accuracy>(weapon).map(|a| **a),
        Some(5.0),
        "an un-attached weapon keeps its authored Accuracy (no Aim effect applied)",
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
/// `SuppressionApplied`. The player's weapon carries `effects`.
fn suppression_probe_app(effects: Vec<AttachmentEffect>) -> (App, Entity) {
    let mut app = battle_app(effects);
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
    let (mut quiet, quiet_shooter) = suppression_probe_app(vec![AttachmentEffect::Silence]);
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
    let (mut silenced_app, silenced_shooter) =
        suppression_probe_app(vec![AttachmentEffect::Silence]);
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
