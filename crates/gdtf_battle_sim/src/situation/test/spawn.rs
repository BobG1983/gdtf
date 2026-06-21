//! Spawn-loop tests — entity count, the full component set, per-field readback,
//! attribute-stat seeding, worn-armor relating, and weapon arming.

use super::support::*;
// The per-piece armor stat newtypes read off the related piece entities (GTW-323
// slice 3) — not re-exported by `support` (which carries only `Wears`/`BodyPart`).
use crate::armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType};

/// C8(a) — the correct entity COUNT spawned: a 2-ganger fixture spawns exactly
/// two ganger entities (each carrying the `Wears` armor relationship) and no more.
#[test]
fn setup_spawns_exactly_the_authored_ganger_count() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    assert_eq!(setup.ganger_count(), 2, "two authored gangers were spawned");

    // Exactly two gangers carry the per-ganger `Wears` armor relationship — the spawned
    // set. Since GTW-323 slice 3 (ADR-0004) the armor stats live on related piece
    // entities, NOT a `WornArmor` component on the ganger, so a ganger is identified by
    // its `Wears` collection.
    let world: &mut World = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        2,
        "exactly two ganger entities exist in the world (each wearing armor via Wears)",
    );
}

/// C8(b) — each spawned ganger carries ALL required components (E1.2 set + the `Wears`
/// armor relationship), proven by a full-tuple query matching both entities.
#[test]
fn each_spawned_ganger_has_all_required_components() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, _setup)) = run_setup(situation) else {
        return;
    };

    let world: &mut World = app.world_mut();
    // A query naming EVERY required component — only an entity carrying all of
    // them matches, so a count of 2 proves both gangers have the full set
    // (E1.2 state + E3.0 attribute stats + the `Wears` armor relationship — the armor
    // STATS themselves live on the related piece entities, GTW-323 slice 3, NOT here).
    // The vitals pools + their display ceilings are grouped into a nested sub-tuple:
    // Bevy's `QueryData` tuple impls cap at 16 elements, and the full set now numbers
    // 17 (GTW-291 added `HpMax` / `WoundsMax`), so nesting keeps the outer arity legal
    // while still requiring every component to match.
    let mut all = world.query::<(
        &Position,
        &GangerName,
        &Faction,
        &Facing,
        &Stance,
        &Aiming,
        (&Hp, &HpMax, &Wounds, &WoundsMax),
        &Tu,
        &TuMax,
        &LifeState,
        &Shooting,
        &Toughness,
        &Luck,
        &Wears,
    )>();
    assert_eq!(
        all.iter(world).count(),
        2,
        "both gangers must carry the full E1.2 set (incl. TuMax + HpMax + WoundsMax + GangerName) \
         + E3.0 attribute stats + the Wears armor relationship",
    );
}

/// C8(c) — each ganger's Position and Faction match the fixture, looked up by
/// the spawned Entity handle (never a numeric id).
#[test]
fn spawned_position_and_faction_match_the_fixture() {
    let (situation, alice_at, bob_at, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    // The setup returns placements in authored order: alice (faction 0) then
    // bob (faction 1).
    let placements = &setup.occupants;
    assert_eq!(placements.len(), 2);

    let world: &mut World = app.world_mut();
    let mut q = world.query::<(&Position, &Faction)>();

    // Alice — placement 0.
    let alice: Entity = placements[0].occupant;
    assert_eq!(placements[0].at, alice_at);
    let alice_components = q.get(world, alice);
    assert!(
        alice_components.is_ok(),
        "alice's spawned entity must carry Position + Faction",
    );
    let Ok((alice_pos, alice_faction)) = alice_components else {
        return;
    };
    assert_eq!(
        *alice_pos,
        Position::new(alice_at),
        "alice Position matches"
    );
    assert_eq!(*alice_faction, Faction::new(0), "alice Faction matches");

    // Bob — placement 1.
    let bob: Entity = placements[1].occupant;
    assert_eq!(placements[1].at, bob_at);
    let bob_components = q.get(world, bob);
    assert!(
        bob_components.is_ok(),
        "bob's spawned entity must carry Position + Faction",
    );
    let Ok((bob_pos, bob_faction)) = bob_components else {
        return;
    };
    assert_eq!(*bob_pos, Position::new(bob_at), "bob Position matches");
    assert_eq!(*bob_faction, Faction::new(1), "bob Faction matches");

    // The two are distinct Entity handles.
    assert_ne!(alice, bob, "the two gangers are distinct entities");
}

/// GTW-285 — `setup_battle` seeds each authored `GangerSpawn.name` onto the spawned
/// ganger as a queryable `GangerName` component, looked up by the spawned `Entity`
/// handle (never a numeric id). Reads BOTH gangers — distinct authored names ("Ganger 0"
/// / "Ganger 1") prove the per-ganger seed, not a shared default. Pin-discriminating:
/// dropping the `name` spawn in `setup_battle` leaves no `GangerName` and this fails.
#[test]
fn setup_seeds_ganger_name_onto_each_ganger() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    let mut q = world.query::<&GangerName>();

    // Alice — faction 0 → the fixture's "Ganger 0" name.
    let alice_name = q.get(world, alice);
    assert_eq!(
        alice_name.map(|n| (**n).clone()).ok(),
        Some("Ganger 0".to_owned()),
        "alice carries her authored GangerName",
    );

    // Bob — faction 1 → the fixture's "Ganger 1" name (a distinct per-ganger seed).
    let bob_name = q.get(world, bob);
    assert_eq!(
        bob_name.map(|n| (**n).clone()).ok(),
        Some("Ganger 1".to_owned()),
        "bob carries his authored GangerName",
    );
}

/// GTW-291 — `setup_battle` seeds each authored `GangerSpawn.hp_max` / `wounds_max`
/// onto the spawned ganger as queryable `HpMax` / `WoundsMax` components, looked up by
/// the spawned `Entity` handle (never a numeric id). The fixture authors both ceilings
/// = the authored full `hp` (40) / `wounds` (3) for every ganger (full at battle start),
/// so a match against `HpMax::new(40)` / `WoundsMax::new(3)` proves the per-ganger seed.
/// Pin-discriminating: dropping the `hp_max` / `wounds_max` spawn in `setup_battle` (the
/// second `insert`) leaves no `HpMax` / `WoundsMax` and this fails. These are DISPLAY
/// ceilings, not reset targets — the test reads them at setup, never after a round flip.
#[test]
fn setup_seeds_hp_max_and_wounds_max_onto_each_ganger() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    let mut q = world.query::<(&HpMax, &WoundsMax)>();

    // Alice — the fixture's authored full capacity (HpMax = hp = 40, WoundsMax = wounds = 3).
    let alice_caps = q.get(world, alice);
    assert_eq!(
        alice_caps.map(|(h, w)| (*h, *w)).ok(),
        Some((HpMax::new(40), WoundsMax::new(3))),
        "alice carries her authored HpMax + WoundsMax display ceilings",
    );

    // Bob — the same authored full capacity (a per-ganger seed, not a shared default).
    let bob_caps = q.get(world, bob);
    assert_eq!(
        bob_caps.map(|(h, w)| (*h, *w)).ok(),
        Some((HpMax::new(40), WoundsMax::new(3))),
        "bob carries his authored HpMax + WoundsMax display ceilings",
    );
}

/// GTW-279 AC2 — `setup_battle` seeds an EMPTY `InflictedWounds` record onto every
/// spawned ganger (alongside the existing vitals), queryable off the spawned `Entity`
/// handle. Reads BOTH gangers — each starts with no recorded wounds. Pin-discriminating:
/// dropping the `InflictedWounds::default()` seed leaves no component and `get` errs.
#[test]
fn setup_seeds_empty_inflicted_wounds_onto_each_ganger() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    let mut q = world.query::<&InflictedWounds>();

    for (who, entity) in [("alice", alice), ("bob", bob)] {
        let record = q.get(world, entity);
        assert!(
            record.is_ok(),
            "{who} must carry a seeded InflictedWounds component (GTW-279 AC2)",
        );
        assert!(
            record.is_ok_and(|r| r.is_empty()),
            "{who}'s InflictedWounds must be seeded EMPTY (no wounds until inflicted)",
        );
    }
}

/// GTW-182 AC #2 + AC #3 — `setup_battle` seeds the E3.0 attribute stats
/// (`Shooting`/`Toughness`/`Luck`) onto each spawned ganger from the authored
/// `GangerSpawn`, and they are queryable off the entity by its spawned `Entity`
/// handle (the read shape the severity roll uses). Reads BOTH gangers — a shooter
/// (faction 0) and a defender (faction 1) — proving the per-ganger seed, not a
/// shared default. Magnitudes match the fixture (`faction + {2,3,1}`), per-ganger
/// data, not pinned tuning.
#[test]
fn setup_seeds_attribute_stats_onto_each_ganger() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    // The severity-roll read shape: a tuple query over the three attribute stats.
    let mut q = world.query::<(&Shooting, &Toughness, &Luck)>();

    // Alice — faction 0 → Shooting 2.0 / Toughness 3.0 / Luck 1.0 (the fixture).
    let alice_stats = q.get(world, alice);
    assert_eq!(
        alice_stats,
        Ok((&Shooting::new(2.0), &Toughness::new(3.0), &Luck::new(1.0))),
        "alice carries her authored Shooting/Toughness/Luck",
    );

    // Bob — faction 1 → Shooting 3.0 / Toughness 4.0 / Luck 2.0 (distinct seed).
    let bob_stats = q.get(world, bob);
    assert_eq!(
        bob_stats,
        Ok((&Shooting::new(3.0), &Toughness::new(4.0), &Luck::new(2.0))),
        "bob carries his authored Shooting/Toughness/Luck",
    );
}

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
        .map(|spec| spec.into_bundle(weapon_key.clone()).damage_type);
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
    assert_eq!(*hp, Hp::new(40), "Hp round-trips");
    assert_eq!(*hp_max, HpMax::new(40), "HpMax round-trips");
    assert_eq!(*wounds, Wounds::new(3), "Wounds round-trips");
    assert_eq!(*wounds_max, WoundsMax::new(3), "WoundsMax round-trips");
    assert_eq!(*cur_tu, Tu::new(60), "Tu round-trips");
    assert_eq!(*tu_max, TuMax::new(60), "TuMax round-trips");
    assert_eq!(
        *life,
        LifeState::Alive,
        "LifeState (template_value) round-trips"
    );
    assert_eq!(*shooting, Shooting::new(2.0), "Shooting round-trips");
    assert_eq!(*toughness, Toughness::new(3.0), "Toughness round-trips");
    assert_eq!(*luck, Luck::new(1.0), "Luck round-trips");

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
