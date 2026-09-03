use bevy::ecs::relationship::{Relationship, RelationshipTarget};

use super::support::*;
use crate::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, Wears, WornBy},
    ganger::{Aim, Cool, GangMember, Grit, Reflexes, Speed, Strength},
    test_support::TEST_ARMOR_KEY,
    weapon::{MeleeWeapon, MountedWeapon},
};

/// One hand-built roster member holding the loadout keys the case needs.
fn member(name: &str, armor: Option<ArmorName>, weapon: Option<WeaponName>) -> GangMember {
    GangMember {
        name: GangerName::new(name.to_owned()),
        speed: Speed::new(3.0),
        aim: Aim::new(2.0),
        strength: Strength::new(3.0),
        toughness: Toughness::new(10.0),
        reflexes: Reflexes::new(2.0),
        cool: Cool::new(8.0),
        grit: Grit::new(18.0),
        luck: Luck::new(1.0),
        armor,
        weapon,
        melee_weapon: None,
    }
}

/// Two faction-0 gangers whose roster members carry the given loadouts, set up for real.
fn two_member_setup(
    first: GangMember,
    second: GangMember,
) -> Option<(bevy::app::App, crate::situation::BattleSetup)> {
    let situation = SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .faction(Faction::new(0))
                .at(key(5, 6, 0))
                .name(first.name.clone())
                .build(),
            GangerSpawnBuilder::new()
                .faction(Faction::new(0))
                .at(key(7, 6, 0))
                .name(second.name.clone())
                .build(),
        ])
        .build();
    // `gang_name_for` is private to test_support, so the key is spelled here.
    let gangs = GangRegistry::new([(
        GangName::new("gang_0".to_owned()),
        GangRoster::new([first, second]),
    )]);
    run_setup_with(
        situation,
        gangs,
        test_registry(),
        test_armor_registry(),
        None,
    )
}

/// The armor key `test_armor_registry` holds.
fn held_armor() -> ArmorName {
    ArmorName::new(TEST_ARMOR_KEY.to_owned())
}

/// The weapon key `test_weapon_registry` holds.
fn held_weapon() -> WeaponName {
    WeaponName::new(TEST_WEAPON_KEY.to_owned())
}

#[test]
fn a_member_holding_no_armor_wears_none_and_the_next_member_keeps_its_own() {
    let Some((app, setup)) = two_member_setup(
        member("Bare", None, Some(held_weapon())),
        member("Clad", Some(held_armor()), Some(held_weapon())),
    ) else {
        return;
    };
    assert_eq!(setup.occupants.len(), 2, "both authored gangers must spawn");
    let (Some(bare), Some(clad)) = (setup.occupants.first(), setup.occupants.get(1)) else {
        return;
    };

    let clad_pieces = app
        .world()
        .get::<Wears>(clad.occupant)
        .map_or(0, |wears| wears.iter().count());
    assert_eq!(
        clad_pieces, 6,
        "the second member wears the six body-part pieces of its own armor spec",
    );
    let bare_pieces = app
        .world()
        .get::<Wears>(bare.occupant)
        .map_or(0, |wears| wears.iter().count());
    assert_eq!(
        bare_pieces, 0,
        "the member holding no armor key wears no pieces",
    );
}

#[test]
fn a_member_holding_no_weapon_still_wields_its_melee_and_fires_nothing() {
    let Some((app, setup)) = two_member_setup(
        member("Unarmed", Some(held_armor()), None),
        member("Armed", Some(held_armor()), Some(held_weapon())),
    ) else {
        return;
    };
    assert_eq!(setup.occupants.len(), 2, "both authored gangers must spawn");
    let (Some(unarmed), Some(armed)) = (setup.occupants.first(), setup.occupants.get(1)) else {
        return;
    };
    let is_melee = |entity| app.world().get::<MeleeWeapon>(entity).is_some();
    let is_mounted = |entity| app.world().get::<MountedWeapon>(entity).is_some();

    let held = app.world().get::<Wields>(unarmed.occupant);
    assert!(
        held.is_some(),
        "the member holding no ranged key still wields its melee weapon",
    );
    let Some(held) = held else { return };
    assert!(
        held.melee_weapon(is_melee).is_some(),
        "that member's `Wields` holds a melee weapon entity",
    );
    assert_eq!(
        held.firing_weapon(is_mounted, is_melee),
        None,
        "the member holding no ranged key fires nothing",
    );

    let armed_wields = app.world().get::<Wields>(armed.occupant);
    assert!(
        armed_wields.is_some_and(|wields| wields.firing_weapon(is_mounted, is_melee).is_some()),
        "the second member keeps its own ranged weapon",
    );
}

fn single_ganger_setup() -> Option<(bevy::app::App, crate::situation::BattleSetup)> {
    let situation = SituationBuilder::new()
        .with_gangers([ganger_at(key(5, 6, 0), 0)])
        .build();
    run_setup(situation)
}

fn expected_piece() -> crate::armor::ArmorPiece {
    crate::test_support::test_armor_spec().pieces()[BodyPart::Head.index()]
}

#[test]
fn setup_relates_six_body_part_tagged_pieces_to_the_ganger() {
    let Some((app, setup)) = single_ganger_setup() else {
        return;
    };
    assert!(
        !setup.occupants.is_empty(),
        "the single-ganger setup must spawn one occupant",
    );
    let Some(placement) = setup.occupants.first() else {
        return;
    };
    let ganger = placement.occupant;

    let wears = app.world().get::<Wears>(ganger);
    assert!(
        wears.is_some(),
        "the spawned ganger must carry a `Wears` relationship-target collection",
    );
    let Some(wears) = wears else { return };
    let pieces: Vec<_> = wears.iter().collect();
    assert_eq!(
        pieces.len(),
        6,
        "the ganger must wear exactly six armor-piece entities (one per BodyPart)",
    );

    let mut tags: Vec<BodyPart> = Vec::new();
    for &piece in &pieces {
        let worn_by = app.world().get::<WornBy>(piece);
        assert!(
            worn_by.is_some_and(|w| w.get() == ganger),
            "each related piece must carry `WornBy(ganger)` pointing at the wearer",
        );
        if let Some(part) = app.world().get::<BodyPart>(piece) {
            tags.push(*part);
        }
    }
    for part in BodyPart::ALL {
        assert!(
            tags.contains(&part),
            "the worn pieces must tag every BodyPart — {part:?} is missing",
        );
    }
}

#[test]
fn each_worn_piece_carries_its_resolved_stat_components() {
    let Some((app, setup)) = single_ganger_setup() else {
        return;
    };
    assert!(
        !setup.occupants.is_empty(),
        "the single-ganger setup must spawn one occupant",
    );
    let Some(placement) = setup.occupants.first() else {
        return;
    };
    let ganger = placement.occupant;
    let wears = app.world().get::<Wears>(ganger);
    assert!(
        wears.is_some(),
        "the spawned ganger must carry a `Wears` collection",
    );
    let Some(wears) = wears else { return };

    let want = expected_piece();
    for piece in wears.iter() {
        let floor = app.world().get::<ArmorFloor>(piece).map(|c| **c);
        let protection = app.world().get::<ArmorProtection>(piece).map(|c| **c);
        let integrity = app.world().get::<ArmorIntegrity>(piece).map(|c| **c);
        let hardness = app.world().get::<ArmorHardness>(piece).map(|c| **c);
        let armor_type = app.world().get::<ArmorType>(piece).copied();
        assert_eq!(
            floor,
            Some(*want.floor),
            "the piece entity must carry the resolved ArmorFloor component",
        );
        assert_eq!(
            protection,
            Some(*want.protection),
            "the piece entity must carry the resolved ArmorProtection component",
        );
        assert_eq!(
            integrity,
            Some(*want.integrity),
            "the piece entity must carry the resolved ArmorIntegrity component",
        );
        assert_eq!(
            hardness,
            Some(*want.hardness),
            "the piece entity must carry the resolved ArmorHardness component",
        );
        assert_eq!(
            armor_type,
            Some(want.armor_type),
            "the piece entity must carry the resolved ArmorType component",
        );
    }
}
