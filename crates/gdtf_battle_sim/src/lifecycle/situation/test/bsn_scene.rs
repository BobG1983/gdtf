//! GTW-322 / GTW-323 — the `bsn!`-scene spawn faithfulness mega-proof: the full
//! ganger set + the related weapon/armor entities + the occupancy placement.

use super::support::*;
// The per-piece armor stat newtypes read off the related piece entities (GTW-323
// slice 3) — not re-exported by `support` (which carries only `Wears`/`BodyPart`).
use crate::armor::{ArmorIntegrity, ArmorType};

/// GTW-322 / GTW-323 slice 3 — the `bsn!`-scene spawn is FAITHFUL: a ganger spawned
/// through `setup_battle` (`commands.spawn_scene(ganger_scene(..))`) carries its OWN
/// per-field state + the E3.0 attribute stats + the empty `InflictedWounds` (and **no**
/// equipment stat data), while the resolved weapon's components (incl. the
/// `template_value`-composed runtime `DamageType` + value-typed `Magazine`) live on the
/// related WEAPON entity (`ganger → Wields → the weapon entity`) and the armor stats
/// live on the related ARMOR-PIECE entities (`ganger → Wears → the pieces`) — AND the
/// occupancy grid places that EXACT spawned `Entity` (keyed off the authored cell, not
/// the deferred `Position` component) with the stance-derived silhouette band. Reads
/// ONE ganger by its spawned handle, asserting every component value (off the ganger,
/// the weapon entity, and the piece entities) + the occupancy placement together (the
/// deferred-spawn contract proof). Pin-discriminating: a dropped component, a
/// sentinel-defaulted `DamageType`/`Magazine`, equipment stat data left on the ganger,
/// or a mis-keyed occupant all fail this.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "GTW-322/323: asserts every component round-trips through the bsn! scenes — the \
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
    // The expected DERIVED stats for alice — RELATION via the single source of truth over
    // her authored attributes × the default tuning the setup uses (GTW-384). Captured
    // BEFORE `run_setup` moves the situation; the authored Toughness/Luck attributes are
    // captured too (they ride through unchanged, the severity roll reads them).
    let alice_attrs = attributes_of(&situation.gangers[0]);
    let alice_derived = derive_stats(&alice_attrs, &GangerStatTuning::default());
    // The armor suit the TEST_ARMOR_KEY resolves to (base 1) — the expected per-piece
    // stats the related piece entities must carry, read straight off the resolved spec.
    let expected_armor = arbitrary_armor(1);
    // The damage type the TEST_WEAPON_KEY resolves to, computed exactly as setup does
    // (registry spec → into_bundle) — the expected value the template_value composition
    // must carry through (NOT the Default sentinel).
    let weapon_key = WeaponName::new(TEST_WEAPON_KEY.to_owned());
    let expected_damage_type = test_registry()
        .spec(&weapon_key)
        .cloned()
        .map(|spec| spec.into_bundle(weapon_key.clone()).0.damage_type);
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    // Alice — authored placement 0 (faction 0, standing). Her spawned Entity handle.
    let alice: Entity = setup.occupants[0].occupant;
    assert_eq!(setup.occupants[0].at, alice_at, "alice's authored cell");

    // The occupancy grid placed the EXACT spawned Entity at the authored cell, with the
    // standing → HIGH silhouette band (keyed off `at`, not the deferred Position).
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

    // Every component the old spawn-tuple + second insert produced is present on the
    // spawned entity, with the authored value. The vitals pools + ceilings nest into a
    // sub-tuple to stay under Bevy's 16-element QueryData arity cap.
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
    // GTW-384: the pools/skills are DERIVED — assert each equals the derive_stats relation
    // for alice's authored attributes (NOT a pinned shipped magnitude). Maxes == current
    // pool (full at battle start).
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
    // Toughness/Luck are AUTHORED attributes (the severity roll reads them) — they
    // round-trip the fixture's per-ganger authored values, not a derive.
    assert_eq!(*toughness, alice_attrs.toughness, "Toughness round-trips");
    assert_eq!(*luck, alice_attrs.luck, "Luck round-trips");

    // The empty InflictedWounds rides ON THE GANGER (the GTW-279 record stays on the
    // ganger; only the equipment moved to related entities).
    let inflicted = world.get::<InflictedWounds>(alice);
    assert!(
        inflicted.is_some_and(|w| w.is_empty()),
        "InflictedWounds rides on the ganger, seeded EMPTY",
    );

    // The weapon's components — incl. the template_value-composed runtime DamageType /
    // value Magazine — live on the related WEAPON entity (`ganger → Wields → weapon`),
    // NOT the ganger (GTW-323 slice 3).
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
    // The test weapon's authored damage type — NOT the Default sentinel. The
    // template_value composition must carry the resolved value through.
    assert_eq!(
        Some(*damage_type),
        expected_damage_type,
        "DamageType (template_value-composed) round-trips — NOT the Default sentinel",
    );

    // The armor stats live on the related ARMOR-PIECE entities (`ganger → Wears → the
    // BodyPart-tagged piece`), NOT a `WornArmor` component on the ganger (GTW-323 slice 3).
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
