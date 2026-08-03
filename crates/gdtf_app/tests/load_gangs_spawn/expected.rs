//! GTW-744: the spawn CELLS are no longer authored (the deploy step derives them from the
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

struct Expected {
    name:       &'static str,
            attributes: [f32; 8],
    weapon:     &'static str,
    armor:      &'static str,
            gang:       &'static str,
    faction:    u8,
    life_state: LifeState,
}

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

const fn expected_set() -> [Expected; 4] {
    [
        Expected {
            name:       "Alex Mercer",
            attributes: [3.0, 3.0, 4.0, 12.0, 3.0, 6.0, 19.0, 1.0],
            weapon:     "volatile_charge",
            armor:      "flak_vest",
            gang:       "gang_0",
            faction:    0,
            life_state: LifeState::Alive,
        },
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

fn assert_ganger(world: &World, entity: Entity, want: &Expected) {
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
