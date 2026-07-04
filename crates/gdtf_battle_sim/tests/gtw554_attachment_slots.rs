//! GTW-554 — ATTACHMENT SLOTS, proven on the REAL `setup_battle_on_request` →
//! `BattleSimPlugin` spawn + `apply_pending_attachments` post-spawn path (the
//! `gtw549_attachments.rs` harness shape). The pure fit rule's five contract cases (accept /
//! wrong-slot / cap-1 full / multi-cap up-to / one-over) are unit-covered on the shared
//! resolution seam in `equipment::attachments::fit`; THIS file proves the gate is LIVE on the
//! spawned weapon entities:
//!
//! - **Compatible item applies** — a Sight item on a Sight-declaring weapon raises Accuracy.
//! - **Wrong slot cleanly rejects** — a Counterweight item on a ranged weapon (which declares
//!   no such slot) applies NOTHING and panics nowhere: the C6 emergent class gate.
//! - **Capacity gates the rail** — a capacity-2 Rail fits the first two rail items and
//!   cleanly rejects the third (no eviction).
//! - **Melee weapons have FULL support** — a Counterweight item on a Counterweight-declaring
//!   MELEE weapon raises the spawned melee entity's `WeaponDamage`.
//!
//! Every magnitude is a distinctive inline baseline (arbitrary test data), never a shipped
//! number (the brittle-test rule).

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Accuracy, BaseSpread, Cell, CellLevel, Cool, Faction, FatalBias, Grit, Kickback, MagazineSize,
    Silenced, Speed, Stance, StanceKind, Strength, WeaponPunch,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Direction, Facing},
    magazine::{Magazine, ReloadTu},
    rng::BattleSeed,
    situation::GangerSpawn,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, single_mode, test_armor_registry,
        test_melee_weapon_spec, test_terrain_registry, test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        AimDelta, AttachmentEffect, AttachmentName, AttachmentRegistry, AttachmentSlot,
        AttachmentSpec, FISTS_KEY, FireMode, MeleeWeapon, MeleeWeaponRegistry, MeleeWeaponSpec,
        SlotCapacity, WeaponDamage, WeaponName, WeaponRegistry, WeaponSlots, WeaponSpec,
    },
};

/// An arbitrary seed (determinism is asserted elsewhere).
const SEED: u64 = 0x0554_5107;
const PLAYER: u8 = 0;
const ENEMY: u8 = 1;
/// The ranged weapon key every fixture ganger resolves.
const WEAPON_KEY: &str = "test-weapon";
/// The DISTINCTIVE ranged baselines the effects perturb (arbitrary, not shipped).
const BASE_ACCURACY: f32 = 5.0;
const BASE_MAG: u16 = 20;
/// The DISTINCTIVE melee damage baseline (arbitrary, not shipped).
const BASE_MELEE_DAMAGE: i32 = 9;

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), gdtf_battle_sim::Level::new(0))
}

fn name(key: &str) -> AttachmentName {
    AttachmentName::new(key.to_owned())
}

/// An attachment item occupying `slot` with one discriminating `effect`.
fn item(slot: AttachmentSlot, effect: AttachmentEffect) -> AttachmentSpec {
    AttachmentSpec {
        display_name: WeaponName::new("Test Item".to_owned()),
        slot,
        effects: vec![effect],
    }
}

/// The fixture attachment registry: a Sight optic, a Counterweight, and three Rail items —
/// each with a discriminating effect (Aim / Damage / Aim+Silence+ExtraAmmo).
fn attachment_registry() -> AttachmentRegistry {
    AttachmentRegistry::new([
        (
            name("optic"),
            item(
                AttachmentSlot::Sight,
                AttachmentEffect::Aim(AimDelta::new(0.6)),
            ),
        ),
        (
            name("weight"),
            item(
                AttachmentSlot::Counterweight,
                AttachmentEffect::Damage(WeaponDamage::new(4)),
            ),
        ),
        (
            name("rail-aim"),
            item(
                AttachmentSlot::Rail,
                AttachmentEffect::Aim(AimDelta::new(0.3)),
            ),
        ),
        (
            name("rail-can"),
            item(AttachmentSlot::Rail, AttachmentEffect::Silence),
        ),
        (
            name("rail-ammo"),
            item(
                AttachmentSlot::Rail,
                AttachmentEffect::ExtraAmmo(MagazineSize::new(12)),
            ),
        ),
    ])
}

/// A ranged spec declaring `slots` and fitting `keys` — distinctive baselines throughout.
fn ranged_spec(slots: WeaponSlots, keys: Vec<AttachmentName>) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.05),
        accuracy: Accuracy::new(BASE_ACCURACY),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(3.0),
        punch: WeaponPunch::new(10),
        magazine: Magazine::loaded(MagazineSize::new(BASE_MAG), ReloadTu::new(20)),
        fire_mode: FireMode::new(vec![single_mode(0.2, 1)]),
        slots,
        attachments: keys,
        ..test_weapon_spec()
    }
}

/// A melee (fists-default) spec declaring `slots` and fitting `keys` — the melee mirror.
fn melee_spec(slots: WeaponSlots, keys: Vec<AttachmentName>) -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        damage: WeaponDamage::new(BASE_MELEE_DAMAGE),
        slots,
        attachments: keys,
        ..test_melee_weapon_spec()
    }
}

/// Build the live-runtime harness with the chosen ranged + melee slot/fitting loadouts, drive
/// one setup, and settle the deferred spawn + post-spawn apply cascade.
fn spawn_battle(
    ranged: (WeaponSlots, Vec<AttachmentName>),
    melee: (WeaponSlots, Vec<AttachmentName>),
) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(12),
        ..Default::default()
    });
    app.insert_resource(WeaponRegistry::new([(
        WeaponName::new(WEAPON_KEY.to_owned()),
        ranged_spec(ranged.0, ranged.1),
    )]));
    app.insert_resource(MeleeWeaponRegistry::new([(
        WeaponName::new(FISTS_KEY.to_owned()),
        melee_spec(melee.0, melee.1),
    )]));
    app.insert_resource(attachment_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());

    let (situation, gangs) = SituationBuilder::new()
        .with_gangers([
            standing(ground(5, 5), PLAYER, Direction::East),
            standing(ground(20, 20), ENEMY, Direction::West),
        ])
        .build_with_gangs();
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    // Setup(Update) → weapon scenes(SpawnScene) → apply_pending_attachments(next Update) →
    // queued attach_to_weapon flushes: a generous settle window (the gtw549 shape).
    for _ in 0..8 {
        app.update();
    }
    app
}

/// A standing ganger of `faction` at `at` facing `dir`.
fn standing(at: CellLevel, faction: u8, dir: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(dir))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(12.0))
        .strength(Strength::new(12.0))
        .grit(Grit::new(12.0))
        .cool(Cool::new(12.0))
        .build()
}

/// The player's spawned RANGED weapon entity (the wielded entity carrying `Accuracy`).
fn player_ranged_weapon(app: &mut App) -> Option<Entity> {
    use bevy::ecs::relationship::Relationship;
    let world = app.world_mut();
    let mut gangers = world.query::<(Entity, &Faction)>();
    let player = gangers
        .iter(world)
        .find(|(_, f)| ***f == PLAYER)
        .map(|(entity, _)| entity)?;
    let mut weapons = world.query::<(Entity, &gdtf_battle_sim::weapon::WieldedBy, &Accuracy)>();
    weapons
        .iter(world)
        .find(|(_, wielded, _)| wielded.get() == player)
        .map(|(entity, ..)| entity)
}

/// The player's spawned MELEE weapon entity (the wielded entity carrying `MeleeWeapon`).
fn player_melee_weapon(app: &mut App) -> Option<Entity> {
    use bevy::ecs::relationship::Relationship;
    let world = app.world_mut();
    let mut gangers = world.query::<(Entity, &Faction)>();
    let player = gangers
        .iter(world)
        .find(|(_, f)| ***f == PLAYER)
        .map(|(entity, _)| entity)?;
    let mut weapons = world.query::<(
        Entity,
        &gdtf_battle_sim::weapon::WieldedBy,
        &MeleeWeapon,
        &WeaponDamage,
    )>();
    weapons
        .iter(world)
        .find(|(_, wielded, ..)| wielded.get() == player)
        .map(|(entity, ..)| entity)
}

/// One declared slot at `capacity`.
fn one_slot(slot: AttachmentSlot, capacity: u8) -> WeaponSlots {
    WeaponSlots::new(vec![(slot, SlotCapacity::new(capacity))])
}

// ── Compatible item applies on the live spawn ─────────────────────────────────────

#[test]
fn compatible_sight_item_applies_on_the_spawned_weapon() {
    let mut app = spawn_battle(
        (one_slot(AttachmentSlot::Sight, 1), vec![name("optic")]),
        (WeaponSlots::default(), Vec::new()),
    );
    let Some(weapon) = player_ranged_weapon(&mut app) else {
        unreachable!("the player wields a ranged weapon entity");
    };
    let accuracy = app.world().get::<Accuracy>(weapon).map(|a| **a);
    assert!(
        accuracy.is_some_and(|a| a > BASE_ACCURACY),
        "the Sight-slotted optic FITS the declared Sight slot and raises Accuracy above the \
         {BASE_ACCURACY} baseline (got {accuracy:?})",
    );
}

// ── Wrong slot: the C6 emergent class gate, live ──────────────────────────────────

#[test]
fn counterweight_is_cleanly_rejected_by_a_ranged_weapon() {
    // The ranged weapon declares ranged-style slots (Sight only here) — the melee-style
    // Counterweight item finds NO slot: clean rejection (nothing applied, no panic), with NO
    // class tag anywhere on the item.
    let mut app = spawn_battle(
        (one_slot(AttachmentSlot::Sight, 1), vec![name("weight")]),
        (WeaponSlots::default(), Vec::new()),
    );
    let Some(weapon) = player_ranged_weapon(&mut app) else {
        unreachable!("the player wields a ranged weapon entity");
    };
    assert_eq!(
        app.world().get::<WeaponDamage>(weapon).map(|d| **d),
        Some(12),
        "the Counterweight item is REJECTED by a weapon with no Counterweight slot — the \
         damage stays at its authored baseline (class gating emerges from slots)",
    );
}

// ── Capacity gates the rail (up to capacity + one over), live ─────────────────────

#[test]
fn rail_capacity_gates_the_third_item() {
    // A capacity-2 Rail with THREE authored rail items: the first two (Aim + Silence) fit,
    // the third (ExtraAmmo) is one OVER capacity — cleanly rejected, no eviction.
    let mut app = spawn_battle(
        (
            one_slot(AttachmentSlot::Rail, 2),
            vec![name("rail-aim"), name("rail-can"), name("rail-ammo")],
        ),
        (WeaponSlots::default(), Vec::new()),
    );
    let Some(weapon) = player_ranged_weapon(&mut app) else {
        unreachable!("the player wields a ranged weapon entity");
    };
    let world = app.world();
    let accuracy = world.get::<Accuracy>(weapon).map(|a| **a);
    assert!(
        accuracy.is_some_and(|a| a > BASE_ACCURACY),
        "rail item 1 of 2 (Aim) FITS and raises Accuracy (got {accuracy:?})",
    );
    assert!(
        world.get::<Silenced>(weapon).is_some(),
        "rail item 2 of 2 (Silence) FITS and fits the Silenced tag",
    );
    assert_eq!(
        world.get::<Magazine>(weapon).map(|m| m.size().get()),
        Some(BASE_MAG),
        "rail item 3 (ExtraAmmo) is ONE OVER the rail's capacity — cleanly rejected, the \
         magazine stays at its {BASE_MAG} baseline (no eviction of a fitted item)",
    );
}

// ── Melee weapons have FULL attachment support, live ──────────────────────────────

#[test]
fn counterweight_applies_on_the_spawned_melee_weapon() {
    // The melee (fists-default) weapon declares a Counterweight socket and fits the weight —
    // the spawned MELEE entity's damage rises (the ranged weapon is untouched).
    let mut app = spawn_battle(
        (WeaponSlots::default(), Vec::new()),
        (
            one_slot(AttachmentSlot::Counterweight, 1),
            vec![name("weight")],
        ),
    );
    let Some(melee) = player_melee_weapon(&mut app) else {
        unreachable!("the player wields a melee weapon entity");
    };
    let damage = app.world().get::<WeaponDamage>(melee).map(|d| **d);
    assert!(
        damage.is_some_and(|d| d > BASE_MELEE_DAMAGE),
        "the Counterweight FITS the melee weapon's declared socket and raises its damage \
         above the {BASE_MELEE_DAMAGE} baseline (got {damage:?}) — melee attachments are \
         fully supported on the real spawn path",
    );
}
