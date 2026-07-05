//! Registry-resolved equipment landing on related entities (GTW-257 / GTW-269 /
//! GTW-323 slice 3) — worn armor pieces + the wielded weapon.

use super::support::*;
// The per-piece armor stat newtypes read off the related piece entities (GTW-323
// slice 3) — not re-exported by `support` (which carries only `Wears`/`BodyPart`).
use crate::armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType};

/// C8(d) / GTW-269 / GTW-323 slice 3 — the worn armor on a spawned ganger equals the
/// suit the `TEST_ARMOR_KEY` resolves to in the registry, field-by-field across all six
/// parts — read off the related ARMOR-PIECE ENTITIES (`ganger → Wears → the
/// BodyPart-tagged piece`), NOT a `WornArmor` component on the ganger (removed in slice
/// 3). The ganger authors only the armor KEY, and setup resolves it into the per-piece
/// stat components on the related entities.
#[test]
fn spawned_worn_armor_matches_the_resolved_registry_spec() {
    use bevy::ecs::relationship::RelationshipTarget;

    let (situation, ..) = minimal_fixture();
    // The suit the test armor registry maps TEST_ARMOR_KEY to (base 1) — the expected
    // per-piece stats the related entities must carry, read straight off the spec.
    let expected = arbitrary_armor(1);
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let world: &mut World = app.world_mut();
    // Resolve `ganger → Wears → the BodyPart-tagged piece`, then read each piece
    // entity's stat components and compare to the resolved spec's piece for that part.
    let wears = world.get::<Wears>(alice);
    assert!(wears.is_some(), "alice must carry a Wears collection");
    let Some(wears) = wears else { return };
    let pieces: Vec<Entity> = wears.iter().collect();
    for part in BodyPart::ALL {
        let want = expected.pieces()[part.index()];
        // The piece tagged with this BodyPart (the keyed `struck_piece` lookup shape).
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

/// GTW-257 AC2 — `setup_battle` ARMS each ganger from the registry: every spawned
/// ganger entity carries the [`Weapon`] marker + its [`WeaponName`] (= the authored
/// key) + the [`FireMode`] selector. Mirrors the worn-armor assertion (the
/// `each_spawned_ganger_has_all_required_components` precedent), proving the resolved
/// [`WeaponBundle`] landed on the real spawn path.
#[test]
fn setup_arms_each_ganger_from_the_registry() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let world: &mut World = app.world_mut();
    // GTW-323 slice 2 (ADR-0004): the authoritative weapon is the related WEAPON entity
    // (one per ganger, `WieldedBy` + Weapon marker + WeaponName + FireMode). A query
    // naming those + `WieldedBy` matches ONLY weapon entities (not the transient
    // on-ganger copy this slice still keeps), so a count of 2 proves BOTH gangers wield a
    // weapon entity armed from the registry.
    let mut armed = world.query::<(&Weapon, &WeaponName, &FireMode, &WieldedBy)>();
    assert_eq!(
        armed.iter(world).count(),
        2,
        "both gangers must wield a weapon ENTITY carrying the Weapon marker + WeaponName + \
         FireMode (armed from the registry)",
    );

    // The first ganger's wielded weapon (`ganger → Wields → the weapon entity`) carries
    // the authored key as its WeaponName, resolved by the ganger's spawned Entity handle
    // (never a numeric id).
    let alice: Entity = setup.occupants[0].occupant;
    let alice_weapon = world.get::<Wields>(alice).and_then(Wields::weapon);
    let alice_name = alice_weapon.and_then(|w| world.get::<WeaponName>(w));
    assert_eq!(
        alice_name.map(|n| (**n).clone()),
        Some(TEST_WEAPON_KEY.to_owned()),
        "the wielded weapon's WeaponName equals the authored weapon key",
    );
}
