use super::support::*;
use crate::armor::{ArmorIntegrity, ArmorType};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "asserts every component round-trips through the bsn! scenes — the \
              ganger's own set, the wielded weapon entity, the worn piece entities, plus occupancy"
)]
fn bsn_scene_ganger_carries_full_set_and_occupancy_placement() {
    use bevy::ecs::relationship::RelationshipTarget;

    use crate::{
        clearance::silhouette_band,
        cover::HeightBand,
        ganger::{Direction, StanceKind},
        magazine::Magazine,
        occupancy::OccupancyGrid,
        weapon::DamageType,
    };

    let (situation, alice_at, ..) = minimal_fixture();
    let alice_attrs = attributes_of(&situation.gangers[0]);
    let alice_derived = derive_stats(&alice_attrs, &GangerStatTuning::default());
    let expected_armor = arbitrary_armor(1);
    let weapon_key = WeaponName::new(TEST_WEAPON_KEY.to_owned());
    let expected_damage_type = test_registry()
        .spec(&weapon_key)
        .cloned()
        .map(|spec| spec.into_bundle(weapon_key.clone()).0.damage_type);
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    assert_eq!(setup.occupants[0].at, alice_at, "alice's authored cell");

    let world: &mut World = app.world_mut();
    let grid_present = world.get_resource::<OccupancyGrid>().is_some();
    assert!(grid_present, "setup must insert an OccupancyGrid");
    let Some(grid) = world.get_resource::<OccupancyGrid>() else {
        return;
    };
    assert_eq!(
        grid.occupant(&alice_at),
        Some(alice),
        "the occupancy grid must place alice's spawned Entity at her authored cell",
    );
    assert_eq!(
        grid.occupant_band(&alice_at),
        Some(silhouette_band(StanceKind::Standing)),
        "the occupancy grid must record alice's stance-derived silhouette band (HIGH)",
    );
    assert_eq!(
        silhouette_band(StanceKind::Standing),
        HeightBand::High,
        "precondition: a standing ganger's silhouette band is HIGH",
    );

    let mut q = world.query::<(
        &Position,
        &GangerName,
        &Faction,
        &Facing,
        &Stance,
        &Aiming,
        (&Hp, &HpMax, &Wounds, &WoundsMax),
        (&Tu, &TuMax),
        &LifeState,
        (&Shooting, &Toughness, &Luck),
    )>();
    let ganger_set = q.get(world, alice);
    assert!(
        ganger_set.is_ok(),
        "alice's spawned entity must carry the full ganger set",
    );
    let Ok((pos, name, faction, facing, stance, aiming, vitals, tu, life, stats)) = ganger_set
    else {
        return;
    };
    let (hp, hp_max, wounds, wounds_max) = vitals;
    let (cur_tu, tu_max) = tu;
    let (shooting, toughness, luck) = stats;
    assert_eq!(*pos, Position::new(alice_at), "Position round-trips");
    assert_eq!((**name), "Ganger 0".to_owned(), "GangerName round-trips");
    assert_eq!(*faction, Faction::new(0), "Faction round-trips");
    assert_eq!(*facing, Facing::new(Direction::East), "Facing round-trips");
    assert_eq!(
        *stance,
        Stance::new(StanceKind::Standing),
        "Stance round-trips"
    );
    assert_eq!(*aiming, Aiming::new(true), "Aiming round-trips");
    assert_eq!(*hp, alice_derived.hp, "Hp == the derived knock-down pool");
    assert_eq!(
        *hp_max, alice_derived.hp_max,
        "HpMax == the derived Hp (full at start)"
    );
    assert_eq!(
        *wounds, alice_derived.wounds,
        "Wounds == the derived life pool"
    );
    assert_eq!(
        *wounds_max, alice_derived.wounds_max,
        "WoundsMax == the derived Wounds (full at start)",
    );
    assert_eq!(*cur_tu, alice_derived.tu, "Tu == the derived action budget");
    assert_eq!(
        *tu_max, alice_derived.tu_max,
        "TuMax == the derived Tu (full at start)"
    );
    assert_eq!(
        *life,
        LifeState::Alive,
        "LifeState (template_value) round-trips"
    );
    assert_eq!(
        *shooting, alice_derived.shooting,
        "Shooting == the derived skill term"
    );
    assert_eq!(*toughness, alice_attrs.toughness, "Toughness round-trips");
    assert_eq!(*luck, alice_attrs.luck, "Luck round-trips");

    let inflicted = world.get::<InflictedWounds>(alice);
    assert!(
        inflicted.is_some_and(|w| w.is_empty()),
        "InflictedWounds rides on the ganger, seeded EMPTY",
    );

    let weapon = world.get::<Wields>(alice).and_then(Wields::weapon);
    assert!(weapon.is_some(), "alice must wield a weapon entity");
    let Some(weapon) = weapon else { return };
    let mut wq = world.query::<(&Weapon, &WeaponName, &FireMode, &DamageType, &Magazine)>();
    let weapon_set = wq.get(world, weapon);
    assert!(
        weapon_set.is_ok(),
        "alice's WEAPON entity must carry the full weapon set",
    );
    let Ok((_marker, weapon_name, _fire, damage_type, _magazine)) = weapon_set else {
        return;
    };
    assert_eq!(
        (**weapon_name),
        TEST_WEAPON_KEY.to_owned(),
        "WeaponName (the authored key) round-trips on the weapon entity",
    );
    assert_eq!(
        Some(*damage_type),
        expected_damage_type,
        "DamageType (template_value-composed) round-trips — NOT the Default sentinel",
    );

    let wears = world.get::<Wears>(alice);
    assert!(wears.is_some(), "alice must carry a Wears collection");
    let Some(wears) = wears else { return };
    let pieces: Vec<Entity> = wears.iter().collect();
    for part in BodyPart::ALL {
        let want = expected_armor.pieces()[part.index()];
        let tagged = pieces
            .iter()
            .find(|&&e| world.get::<BodyPart>(e) == Some(&part))
            .copied();
        assert!(tagged.is_some(), "no related piece is tagged {part:?}");
        let Some(piece) = tagged else { return };
        assert_eq!(
            world.get::<ArmorIntegrity>(piece).map(|c| **c),
            Some(*want.integrity),
            "worn piece integrity at {part:?} round-trips on the piece entity",
        );
        assert_eq!(
            world.get::<ArmorType>(piece).copied(),
            Some(want.armor_type),
            "worn piece armor_type at {part:?} round-trips on the piece entity",
        );
    }
}
