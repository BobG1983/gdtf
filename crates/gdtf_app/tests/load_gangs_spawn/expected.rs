//! The concrete expected spawn set + its field-for-field assertions — the
//! pre-GTW-414 inline values the migration must have preserved verbatim (the
//! explicit pin-the-migrated-values exception to the no-pinning rule), split
//! from the `load_gangs_spawn.rs` test body under the repo file caps.

use std::collections::HashMap;

use bevy::prelude::{Entity, World};
use gdtf_battle_sim::{
    ganger::{
        Aim, Aiming, Cool, Facing, GangName, GangRegistry, GangerName, Grit, Luck, Reflexes, Speed,
        Strength, Toughness,
    },
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, Position, Stance, StanceKind,
    },
    weapon::{WeaponName, Wields},
};

/// The concrete, expected spawn record for one ganger — the pre-GTW-414 inline value an
/// authored ganger carried, now sourced from the migrated gang roster + situation
/// placement. Compared field-for-field against the spawned entity so the migration's
/// value-preservation is proven exactly (the explicit pin-the-migrated-values exception).
struct Expected {
    name:       &'static str,
    /// The eight direct attributes (`speed, aim, strength, toughness, reflexes, cool,
    /// grit, luck`) — the raw authored potential, verbatim from the gang `.ron`.
    attributes: [f32; 8],
    weapon:     &'static str,
    armor:      &'static str,
    /// The gang the situation references this member through (`gang_0` / `gang_1`) — the
    /// key the data-level weapon + armor assertions resolve the roster member against.
    gang:       &'static str,
    position:   CellLevel,
    faction:    u8,
    facing:     Direction,
    stance:     StanceKind,
    aiming:     bool,
    life_state: LifeState,
}

/// Assert the whole expected shipped set spawned: exactly the expected count, and each
/// expected ganger spawned with its concrete migrated values (placement + attributes +
/// weapon) plus the migrated weapon + armor KEYS (data-level, via the real gang registry).
pub(crate) fn assert_expected_set(
    world: &World,
    spawned: &HashMap<String, Entity>,
    gangs: &GangRegistry,
) {
    let expected = expected_set();
    assert_eq!(
        spawned.len(),
        expected.len(),
        "setup_battle must spawn exactly the {} expected gangers from the real skirmish + gangs",
        expected.len(),
    );
    for want in &expected {
        assert!(
            spawned.contains_key(want.name),
            "expected ganger {:?} must have spawned",
            want.name,
        );
        let Some(&entity) = spawned.get(want.name) else {
            continue;
        };
        assert_ganger(world, entity, want);
        assert_roster_keys(gangs, want);
    }
}

/// The expected shipped ganger set (the migrated gang rosters + situation placements): the
/// three original gangers plus the GTW-546 grenadier "Kira Vann" (a player-faction member of
/// `gang_0` wielding the ARC `grenade_launcher`, so the contextual Throw act is live in the shipped
/// skirmish).
fn expected_set() -> [Expected; 4] {
    [
        Expected {
            name:       "Alex Mercer",
            attributes: [3.0, 3.0, 4.0, 12.0, 3.0, 6.0, 19.0, 1.0],
            // GTW-547: Alex now wields the `volatile_charge` (an on-death Explode weapon) so the
            // on-death Explode effect is LIVE in the shipped skirmish (see gang_0.gang.ron).
            weapon:     "volatile_charge",
            armor:      "flak_vest",
            gang:       "gang_0",
            position:   cell(5, 6, 0),
            faction:    0,
            facing:     Direction::East,
            stance:     StanceKind::Standing,
            aiming:     false,
            life_state: LifeState::Alive,
        },
        // GTW-546: the grenadier — a player-faction (gang_0) member wielding the ARC
        // grenade_launcher, placed beside Alex, so the contextual THROW act is live in the shipped
        // skirmish. Same attributes / flak_vest as Alex (a survivable grenadier).
        Expected {
            name:       "Kira Vann",
            attributes: [3.0, 3.0, 4.0, 12.0, 3.0, 6.0, 19.0, 1.0],
            weapon:     "grenade_launcher",
            armor:      "flak_vest",
            gang:       "gang_0",
            position:   cell(5, 8, 0),
            faction:    0,
            facing:     Direction::East,
            stance:     StanceKind::Standing,
            aiming:     false,
            life_state: LifeState::Alive,
        },
        Expected {
            name:       "Vex 1",
            attributes: [3.0, 4.0, 3.0, 13.0, 3.0, 6.0, 18.0, 2.0],
            weapon:     "las_carbine",
            armor:      "carapace_plate",
            gang:       "gang_1",
            position:   cell(12, 9, 0),
            faction:    1,
            facing:     Direction::West,
            stance:     StanceKind::Crouching,
            aiming:     true,
            life_state: LifeState::Alive,
        },
        Expected {
            name:       "Vex 2",
            attributes: [3.0, 4.0, 3.0, 13.0, 3.0, 6.0, 18.0, 2.0],
            weapon:     "las_carbine",
            armor:      "carapace_plate",
            gang:       "gang_1",
            position:   cell(12, 12, 0),
            faction:    1,
            facing:     Direction::West,
            stance:     StanceKind::Crouching,
            aiming:     true,
            life_state: LifeState::Alive,
        },
    ]
}

/// A `(cell, level)` key for an expected position.
fn cell(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

/// Assert the spawned ganger entity carries the concrete migrated placement + attribute
/// + weapon values from `want` (field-for-field — the migration-preserved-them proof).
fn assert_ganger(world: &World, entity: Entity, want: &Expected) {
    // Placement is components on the ganger entity.
    assert_eq!(
        world.get::<Position>(entity).map(|p| **p),
        Some(want.position),
        "{}: spawned Position must match the migrated placement",
        want.name,
    );
    assert_eq!(
        world.get::<Faction>(entity).map(|f| **f),
        Some(want.faction),
        "{}: spawned Faction must match the situation-assigned side",
        want.name,
    );
    assert_eq!(
        world.get::<Facing>(entity).map(|f| **f),
        Some(want.facing),
        "{}: spawned Facing must match the migrated placement",
        want.name,
    );
    assert_eq!(
        world.get::<Stance>(entity).map(|s| **s),
        Some(want.stance),
        "{}: spawned Stance must match the migrated placement",
        want.name,
    );
    assert_eq!(
        world.get::<Aiming>(entity).map(|a| **a),
        Some(want.aiming),
        "{}: spawned Aiming must match the migrated placement",
        want.name,
    );
    assert_eq!(
        world.get::<LifeState>(entity).copied(),
        Some(want.life_state),
        "{}: spawned LifeState must match the migrated placement",
        want.name,
    );

    // The eight direct attributes (the raw authored potential, verbatim from the roster).
    let attributes = [
        world.get::<Speed>(entity).map(|v| **v),
        world.get::<Aim>(entity).map(|v| **v),
        world.get::<Strength>(entity).map(|v| **v),
        world.get::<Toughness>(entity).map(|v| **v),
        world.get::<Reflexes>(entity).map(|v| **v),
        world.get::<Cool>(entity).map(|v| **v),
        world.get::<Grit>(entity).map(|v| **v),
        world.get::<Luck>(entity).map(|v| **v),
    ];
    for (i, expected) in want.attributes.iter().enumerate() {
        assert_eq!(
            attributes[i],
            Some(*expected),
            "{}: attribute[{i}] must match the migrated roster value {expected}",
            want.name,
        );
    }

    // The wielded weapon: the ganger relates to one weapon entity (ganger → Wields → the
    // weapon entity), whose WeaponName is the resolved key from the migrated roster — so
    // the ganger ends up ARMED from the migrated roster weapon, spawned through the real
    // setup path.
    let weapon_name = world
        .get::<Wields>(entity)
        .and_then(Wields::weapon)
        .and_then(|weapon_entity| world.get::<WeaponName>(weapon_entity))
        .map(|n| (**n).clone());
    assert_eq!(
        weapon_name,
        Some(want.weapon.to_owned()),
        "{}: the wielded weapon entity's WeaponName must be the migrated roster weapon key",
        want.name,
    );
}

/// Data-level weapon + armor key proof: the migrated roster member's weapon + armor keys
/// (resolved through the real gang registry) match the pre-migration inline values. The
/// keys are not spawned components, so they are asserted against the registry the placement
/// resolves through.
fn assert_roster_keys(gangs: &GangRegistry, want: &Expected) {
    let member = gangs
        .roster(&GangName::new(want.gang.to_owned()))
        .and_then(|roster| roster.member(&GangerName::new(want.name.to_owned())));
    assert!(
        member.is_some(),
        "{}: must resolve in the real gang registry",
        want.name,
    );
    let Some(member) = member else {
        return;
    };
    assert_eq!(
        (*member.weapon).as_str(),
        want.weapon,
        "{}: migrated roster weapon key must match the pre-migration value",
        want.name,
    );
    assert_eq!(
        (*member.armor).as_str(),
        want.armor,
        "{}: migrated roster armor key must match the pre-migration value",
        want.name,
    );
}
