//! `Wields` ALONGSIDE its ranged weapon, and a ganger authoring NO melee key resolves to
use super::support::*;
use crate::weapon::{
    DamageType, FightMode, MeleeWeapon, Reach, Weapon, WeaponDamage, WeaponName, Wields,
};

#[test]
fn every_ganger_wields_a_melee_weapon_entity_via_wields() {
    use bevy::ecs::relationship::RelationshipTarget;

    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice = setup.occupants[0].occupant;
    let world: &mut World = app.world_mut();

    let related: Vec<bevy::prelude::Entity> = world
        .get::<Wields>(alice)
        .map(|wields| wields.iter().collect())
        .unwrap_or_default();
    assert_eq!(
        related.len(),
        2,
        "a spawned ganger wields TWO weapon entities (a ranged + a melee), via Wields",
    );

    let melee: Vec<_> = related
        .iter()
        .copied()
        .filter(|&e| world.get::<MeleeWeapon>(e).is_some())
        .collect();
    let ranged: Vec<_> = related
        .iter()
        .copied()
        .filter(|&e| world.get::<Weapon>(e).is_some())
        .collect();
    assert_eq!(
        melee.len(),
        1,
        "exactly one related entity is a MeleeWeapon"
    );
    assert_eq!(
        ranged.len(),
        1,
        "exactly one related entity is a ranged Weapon"
    );

    let Some(&melee_entity) = melee.first() else {
        return;
    };
    assert!(
        world.get::<MeleeWeapon>(melee_entity).is_some(),
        "the melee entity carries the MeleeWeapon marker",
    );
    assert!(
        world.get::<Weapon>(melee_entity).is_none(),
        "the melee entity does NOT carry the ranged Weapon marker (so ranged lookups exclude it)",
    );
    assert!(
        world.get::<WeaponDamage>(melee_entity).is_some(),
        "the melee entity carries the shared WeaponDamage component",
    );
    assert!(
        world.get::<DamageType>(melee_entity).is_some(),
        "the melee entity carries the shared DamageType component",
    );
    assert!(
        world.get::<Reach>(melee_entity).is_some(),
        "the melee entity carries the melee-only Reach component",
    );
    assert!(
        world.get::<FightMode>(melee_entity).is_some(),
        "the melee entity carries the melee-only FightMode component",
    );
    assert!(
        world
            .get::<crate::weapon::BaseSpread>(melee_entity)
            .is_none(),
        "the melee entity must NOT carry the ranged-only BaseSpread (it dropped ranged handling)",
    );
    assert!(
        world
            .get::<crate::magazine::Magazine>(melee_entity)
            .is_none(),
        "the melee entity must NOT carry a Magazine (a melee weapon has no ammo)",
    );
}

#[test]
fn ganger_with_no_melee_key_resolves_the_fists_default() {
    use bevy::ecs::relationship::RelationshipTarget;

    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice = setup.occupants[0].occupant;
    let world: &mut World = app.world_mut();

    let melee_entity = world.get::<Wields>(alice).and_then(|wields| {
        wields
            .iter()
            .find(|&e| world.get::<MeleeWeapon>(e).is_some())
    });
    assert!(
        melee_entity.is_some(),
        "alice must wield a melee weapon entity (the fists default)",
    );
    let Some(melee_entity) = melee_entity else {
        return;
    };
    let name = world.get::<WeaponName>(melee_entity).map(|n| (**n).clone());
    assert_eq!(
        name,
        Some(crate::weapon::FISTS_KEY.to_owned()),
        "a ganger that authors no melee weapon resolves to the `fists` default key",
    );
    assert_eq!(
        crate::weapon::FISTS_KEY,
        "fists",
        "precondition: the fists key is `fists`"
    );
}
