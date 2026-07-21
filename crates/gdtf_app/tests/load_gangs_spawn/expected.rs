//! The concrete expected spawn set + its field-for-field assertions — the
//! pre-GTW-414 inline ROSTER values the migration must have preserved verbatim
//! (identity + attributes + weapon/armor keys + faction), split from the
//! `load_gangs_spawn.rs` test body under the repo file caps.
//!
//! GTW-744: the spawn CELLS are no longer authored (the deploy step derives them from the
//! generated map), so this no longer pins exact positions/facing/stance — only the migration-
//! preserved roster VALUES field-for-field, plus that all four members deployed to distinct,
//! live positions.

use std::collections::HashMap;

use bevy::prelude::{Entity, World};
use gdtf_battle_sim::{
    ganger::{
        Aim, Cool, GangName, GangRegistry, GangerName, Grit, Luck, Position, Reflexes, Speed,
        Strength, Toughness,
    },
    prelude::{Faction, LifeState},
    weapon::{WeaponName, Wields},
};

/// The concrete, expected spawn record for one ganger — the pre-GTW-414 inline ROSTER value an
/// authored ganger carried, now sourced from the migrated gang roster + situation faction.
/// Compared field-for-field against the spawned entity so the migration's value-preservation is
/// proven exactly (the explicit pin-the-migrated-values exception). GTW-744: placement fields
/// (cell / facing / stance / aiming) are DROPPED — the deploy step derives them.
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
    faction:    u8,
    life_state: LifeState,
}

/// Assert the whole expected shipped set spawned: exactly the expected count, each expected
/// ganger spawned with its concrete migrated values (attributes + weapon + faction) plus the
/// migrated weapon + armor KEYS (data-level, via the real gang registry), and every member
/// deployed to a distinct, live position (the procgen deploy step placed them — GTW-744).
pub(crate) fn assert_expected_set(
    world: &World,
    spawned: &HashMap<String, Entity>,
    gangs: &GangRegistry,
) {
    let expected = expected_set();
    assert_eq!(
        spawned.len(),
        expected.len(),
        "the live path must spawn exactly the {} expected gangers from the real skirmish + gangs",
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

    // GTW-744: every member deployed to a distinct, live position (procgen placement).
    let mut positions: Vec<(i32, i32, i32)> = Vec::new();
    for want in &expected {
        let Some(&entity) = spawned.get(want.name) else {
            continue;
        };
        let cell = world.get::<Position>(entity).map(|p| {
            let cl = **p;
            (cl.x, cl.y, cl.z)
        });
        assert!(
            cell.is_some(),
            "{}: must carry a deployed Position component",
            want.name,
        );
        let Some(cell) = cell else {
            continue;
        };
        assert!(
            !positions.contains(&cell),
            "{}: deployed position {cell:?} must be distinct (no two gangers stacked)",
            want.name,
        );
        positions.push(cell);
    }
}

/// The expected shipped ganger set (the migrated gang rosters + situation factions): the three
/// original gangers plus the GTW-546 grenadier "Kira Vann" (a player-faction member of `gang_0`
/// wielding the ARC `grenade_launcher`, so the contextual Throw act is live in the shipped
/// skirmish).
const fn expected_set() -> [Expected; 4] {
    [
        Expected {
            name:       "Alex Mercer",
            attributes: [3.0, 3.0, 4.0, 12.0, 3.0, 6.0, 19.0, 1.0],
            // GTW-547: Alex now wields the `volatile_charge` (an on-death Explode weapon) so the
            // on-death Explode effect is LIVE in the shipped skirmish (see gang_0.gang.ron).
            weapon:     "volatile_charge",
            armor:      "flak_vest",
            gang:       "gang_0",
            faction:    0,
            life_state: LifeState::Alive,
        },
        // GTW-546: the grenadier — a player-faction (gang_0) member wielding the ARC
        // grenade_launcher, so the contextual THROW act is live in the shipped skirmish. Same
        // attributes / flak_vest as Alex (a survivable grenadier).
        Expected {
            name:       "Kira Vann",
            attributes: [3.0, 3.0, 4.0, 12.0, 3.0, 6.0, 19.0, 1.0],
            weapon:     "grenade_launcher",
            armor:      "flak_vest",
            gang:       "gang_0",
            faction:    0,
            life_state: LifeState::Alive,
        },
        Expected {
            name:       "Vex 1",
            attributes: [3.0, 4.0, 3.0, 13.0, 3.0, 6.0, 18.0, 2.0],
            weapon:     "las_carbine",
            armor:      "carapace_plate",
            gang:       "gang_1",
            faction:    1,
            life_state: LifeState::Alive,
        },
        Expected {
            name:       "Vex 2",
            attributes: [3.0, 4.0, 3.0, 13.0, 3.0, 6.0, 18.0, 2.0],
            weapon:     "las_carbine",
            armor:      "carapace_plate",
            gang:       "gang_1",
            faction:    1,
            life_state: LifeState::Alive,
        },
    ]
}

/// Assert the spawned ganger entity carries the concrete migrated ROSTER + faction values from
/// `want` (field-for-field — the migration-preserved-them proof). GTW-744: placement is
/// procgen-derived, so only faction / attributes / weapon / life-state are pinned.
fn assert_ganger(world: &World, entity: Entity, want: &Expected) {
    // Faction is the side the situation assigned (roster-authored in `rosters`, preserved).
    assert_eq!(
        world.get::<Faction>(entity).map(|f| **f),
        Some(want.faction),
        "{}: spawned Faction must match the situation-assigned side",
        want.name,
    );
    assert_eq!(
        world.get::<LifeState>(entity).copied(),
        Some(want.life_state),
        "{}: spawned LifeState must be Alive (the deploy default)",
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
