//! GTW-549 (child GTW-551 → GTW-17) — DATA-DRIVEN WEAPON ATTACHMENTS, the PHASE-5 example /
//! liveness sweep, proven on the REAL `setup_battle_on_request` → `BattleSimPlugin` spawn +
//! `apply_pending_attachments` post-spawn path. Complements `gtw542_weapon_attachments.rs`
//! (which covers Aim / Stability / `ExtraAmmo` / `ReloadTime` / Silence / identity / the
//! Silenced dual-producer gate); this file pins the remaining ticket clauses:
//!
//! - **`GainFireMode` adds a mode** — a `GainFireMode` attachment appends its `FireModeSpec` to
//!   the weapon's `FireMode` selector on the real spawn (the mode count grows by one).
//! - **Aim raises Accuracy** — the headline lever, re-asserted here against a distinctive inline
//!   baseline (a sight boosts AIM, not stability).
//! - **`ExtraAmmo` raises the magazine** — the capacity grows on the real spawn.
//! - **Silence still gates suppression** — a `Silence` attachment makes a point-blank shot
//!   produce NO `SuppressionApplied` where an identical un-silenced shot does (the PRESERVED
//!   producer gate), proving the effect wires the tag both producer gates read.
//! - **Empty-slots identity** — an empty `attachments` list spawns a weapon with no attachment
//!   effects (no added mode, no `Silenced`, un-raised Accuracy).
//! - **Hot-reload** — re-resolving an EDITED attachment spec through the registry changes the
//!   applied stat on the next spawn (the registry is the LIVE source of truth the loader's
//!   hot-reload rebuilds; app-side the `Modified` event drives the rebuild — asserted in
//!   `gdtf_app`'s `resolve::attachments` unit tests).
//!
//! Loader tests do NOT pin shipped magnitudes — every assert checks presence / relative
//! direction / count against a distinctive inline baseline, never a shipped number.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MessageReader, MinimalPlugins, ResMut, Resource},
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
        AimDelta, AttachmentEffect, AttachmentName, AttachmentRegistry, AttachmentSpec, FireMode,
        FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, TrajectoryStyle,
        WeaponDamage, WeaponName, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// An arbitrary seed (determinism is asserted elsewhere).
const SEED: u64 = 0x0549_A77A;
const PLAYER: u8 = 0;
const ENEMY: u8 = 1;
/// A view range covering the point-blank fixtures.
const TEST_VIEW_RANGE: u16 = 12;
/// The weapon key every fixture ganger resolves.
const WEAPON_KEY: &str = "test-weapon";
/// The attachment item key the fixture weapon references.
const ATTACHMENT_KEY: &str = "test-attachment";

const fn level0() -> gdtf_battle_sim::Level {
    gdtf_battle_sim::Level::new(0)
}

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), level0())
}

/// A single-shot mode (the fixture weapon's authored mode).
const fn single_shot_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

/// A ranged weapon spec referencing the chosen attachment KEYS — arbitrary (not shipped)
/// magnitudes; ONE single-shot mode so a `GainFireMode` addition is unambiguously the second.
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
        fire_mode:   FireMode::new(vec![single_shot_mode()]),
        stable:      Stable::new(false),
        shove:       Shove::new(false),
        handedness:  Handedness::OneHanded,
        trajectory:  TrajectoryStyle::Straight,
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
/// `Accuracy` stat (the melee weapon / fists does not).
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

/// A distinctive second mode a `GainFireMode` attachment grants (a full-auto mode absent from
/// the single-mode fixture) — never a shipped magnitude.
const fn granted_burst_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Full,
        ModeConeMult::new(1.7),
        ModeTuPercent::new(0.6),
        ModeShots::new(6),
    )
}

// ── GainFireMode adds a mode ─────────────────────────────────────────────────────

#[test]
fn gain_fire_mode_effect_adds_a_fire_mode() {
    // Baseline: the fixture authors exactly ONE mode.
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base_modes = base_app
        .world()
        .get::<FireMode>(base_weapon)
        .map(|modes| modes.len());
    // A GainFireMode attachment appends a second mode.
    let (gain_app, gain_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::GainFireMode(granted_burst_mode())]);
    let gain_modes = gain_app
        .world()
        .get::<FireMode>(gain_weapon)
        .map(|modes| modes.len());
    let (Some(base_modes), Some(gain_modes)) = (base_modes, gain_modes) else {
        unreachable!("both weapons carry a FireMode selector");
    };
    assert_eq!(
        gain_modes,
        base_modes + 1,
        "a GainFireMode attachment adds one mode to the weapon's selector \
         (gained {gain_modes} = baseline {base_modes} + 1)",
    );
    // The appended mode is the granted one (the Full-auto mode the fixture never authored).
    let has_granted = gain_app
        .world()
        .get::<FireMode>(gain_weapon)
        .is_some_and(|modes| modes.iter().any(|m| *m == granted_burst_mode()));
    assert!(
        has_granted,
        "the added mode is the GainFireMode payload (the granted Full-auto mode)",
    );
}

// ── Aim raises Accuracy (the headline lever) ─────────────────────────────────────

#[test]
fn aim_effect_raises_accuracy() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app.world().get::<Accuracy>(base_weapon).map(|a| **a);
    let (aim_app, aim_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::Aim(AimDelta::new(0.6))]);
    let aimed = aim_app.world().get::<Accuracy>(aim_weapon).map(|a| **a);
    let (Some(base), Some(aimed)) = (base, aimed) else {
        unreachable!("both weapons carry Accuracy");
    };
    assert!(
        aimed > base,
        "an Aim attachment raises the weapon's Accuracy (aimed {aimed} > baseline {base}) — a \
         sight boosts AIM, not stability",
    );
}

// ── ExtraAmmo raises the magazine ────────────────────────────────────────────────

#[test]
fn extra_ammo_effect_grows_the_magazine() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app
        .world()
        .get::<Magazine>(base_weapon)
        .map(|m| m.size().get());
    let (drum_app, drum_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::ExtraAmmo(MagazineSize::new(12))]);
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

// ── Empty-slots identity ─────────────────────────────────────────────────────────

#[test]
fn empty_attachments_spawn_with_no_effects() {
    let (app, weapon) = spawn_lone_player_weapon(Vec::new());
    let world = app.world();
    assert!(
        world.get::<Silenced>(weapon).is_none(),
        "an un-attached weapon has NO Silenced sibling",
    );
    assert_eq!(
        world.get::<FireMode>(weapon).map(|modes| modes.len()),
        Some(1),
        "an un-attached weapon keeps its single authored fire mode (no GainFireMode applied)",
    );
    assert_eq!(
        world.get::<Accuracy>(weapon).map(|a| **a),
        Some(5.0),
        "an un-attached weapon keeps its authored Accuracy (no Aim effect applied)",
    );
}

// ── Silence still gates suppression (the PRESERVED producer gate) ─────────────────

/// A test-local recorder of every `SuppressionApplied` observed across the run.
#[derive(Resource, Default)]
struct AppliedLog {
    /// The number of `SuppressionApplied` signals observed.
    count: usize,
}

/// Drain `SuppressionApplied` into the recorder (registered after `BattleSimPlugin`).
fn record_applied(mut msgs: MessageReader<SuppressionApplied>, mut log: ResMut<AppliedLog>) {
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
fn silence_effect_still_gates_suppression() {
    // Un-silenced control: the adjacent enemy IS suppressed.
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
        "a SILENCED shot produces NO SuppressionApplied (the PRESERVED producer gate reads the \
         shooter's Silenced weapon)",
    );
}

// ── Hot-reload: the registry is the LIVE source the loader rebuilds ──────────────

#[test]
fn editing_the_attachment_registry_changes_the_next_spawn() {
    // The loader's hot-reload rebuilds the AttachmentRegistry in place on a `*.attachment.ron`
    // edit (asserted app-side in gdtf_app's resolve::attachments unit tests). Here we prove the
    // registry IS the live source of truth: an edited spec (larger Aim) applies a stronger
    // effect on the next spawn — so a hot-reloaded item takes effect for the weapons spawned
    // after the rebuild.
    let (small_app, small_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::Aim(AimDelta::new(0.2))]);
    let small = small_app.world().get::<Accuracy>(small_weapon).map(|a| **a);

    // Rebuild the same key with a LARGER Aim (the shape of a hot-reload edit): the loader
    // rebuilds this resource in place on a `*.attachment.ron` `Modified` event; here the test
    // body mutates it directly (the accepted headless idiom), then spawns afresh.
    let mut app = battle_app(vec![AttachmentEffect::Aim(AimDelta::new(0.2))]);
    if let Some(mut registry) = app.world_mut().get_resource_mut::<AttachmentRegistry>() {
        registry.insert(
            AttachmentName::new(ATTACHMENT_KEY.to_owned()),
            AttachmentSpec {
                display_name: WeaponName::new("Edited".to_owned()),
                effects:      vec![AttachmentEffect::Aim(AimDelta::new(0.8))],
            },
        );
    }
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
    let edited = app.world().get::<Accuracy>(weapon).map(|a| **a);

    let (Some(small), Some(edited)) = (small, edited) else {
        unreachable!("both weapons carry Accuracy");
    };
    assert!(
        edited > small,
        "an EDITED attachment spec (the hot-reload shape) applies its stronger effect on the \
         next spawn (edited Accuracy {edited} > pre-edit {small}) — the registry is the live \
         source of truth",
    );
}
