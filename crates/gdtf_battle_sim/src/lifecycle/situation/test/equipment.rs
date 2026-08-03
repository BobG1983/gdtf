use super::support::*;
use crate::armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType};

#[test]
fn spawned_worn_armor_matches_the_resolved_registry_spec() {
    use bevy::ecs::relationship::RelationshipTarget;

    let (situation, ..) = minimal_fixture();
    let expected = arbitrary_armor(1);
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let world: &mut World = app.world_mut();
    let wears = world.get::<Wears>(alice);
    assert!(wears.is_some(), "alice must carry a Wears collection");
    let Some(wears) = wears else { return };
    let pieces: Vec<Entity> = wears.iter().collect();
    for part in BodyPart::ALL {
        let want = expected.pieces()[part.index()];
        let tagged = pieces
            .iter()
            .find(|&&e| world.get::<BodyPart>(e) == Some(&part))
            .copied();
        assert!(tagged.is_some(), "no related piece is tagged {part:?}");
        let Some(piece) = tagged else { return };
        assert_eq!(
            world.get::<ArmorFloor>(piece).map(|c| **c),
            Some(*want.floor),
            "worn piece floor at {part:?} must equal the registry-resolved piece",
        );
        assert_eq!(
            world.get::<ArmorProtection>(piece).map(|c| **c),
            Some(*want.protection),
            "worn piece protection at {part:?} must equal the registry-resolved piece",
        );
        assert_eq!(
            world.get::<ArmorIntegrity>(piece).map(|c| **c),
            Some(*want.integrity),
            "worn piece integrity at {part:?} must equal the registry-resolved piece",
        );
        assert_eq!(
            world.get::<ArmorHardness>(piece).map(|c| **c),
            Some(*want.hardness),
            "worn piece hardness at {part:?} must equal the registry-resolved piece",
        );
        assert_eq!(
            world.get::<ArmorType>(piece).copied(),
            Some(want.armor_type),
            "worn piece armor_type at {part:?} must equal the registry-resolved piece",
        );
    }
}

#[test]
fn setup_arms_each_ganger_from_the_registry() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let world: &mut World = app.world_mut();
    let mut armed = world.query::<(&Weapon, &WeaponName, &FireMode, &WieldedBy)>();
    assert_eq!(
        armed.iter(world).count(),
        2,
        "both gangers must wield a weapon ENTITY carrying the Weapon marker + WeaponName + \
         FireMode (armed from the registry)",
    );

    let alice: Entity = setup.occupants[0].occupant;
    let alice_weapon = world.get::<Wields>(alice).and_then(Wields::weapon);
    let alice_name = alice_weapon.and_then(|w| world.get::<WeaponName>(w));
    assert_eq!(
        alice_name.map(|n| (**n).clone()),
        Some(TEST_WEAPON_KEY.to_owned()),
        "the wielded weapon's WeaponName equals the authored weapon key",
    );
}
