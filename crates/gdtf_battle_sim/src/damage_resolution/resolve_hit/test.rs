use super::{formula::scale_by_matchup, *};
use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        BodyPart, SourceArmor,
    },
    matchup::{Matchup, MatchupMultiplier, matchup_multiplier},
    tuning::CombatTuning,
    weapon::{WeaponDamage, WeaponPunch, WeaponShred},
};

fn armor_piece(floor: i32, protection: i32, hardness: i32) -> ArmorPiece {
    ArmorPiece::new(
        ArmorFloor::new(floor),
        ArmorProtection::new(protection),
        ArmorIntegrity::new(100),
        ArmorHardness::new(hardness),
        ArmorType::DEFAULT,
    )
}

#[test]
fn hp_damage_clamps_up_to_floor() {
    let tuning = CombatTuning::default();
    let armor = armor_piece(4, 20, 0);
    let result = resolve_hit(
        WeaponDamage::new(3),
        WeaponPunch::new(0),
        WeaponShred::new(0),
        &armor,
        Matchup::Neutral,
        &tuning,
    );

    assert_eq!(
        *result.hp_damage, *armor.floor,
        "HP-loss damage must clamp up to the armor floor when inner < floor",
    );
    assert!(
        *result.hp_damage >= *armor.floor,
        "HP-loss damage must never drop below the floor",
    );
    assert_eq!(
        *result.penetrating, 0,
        "a fully-soaked hit must penetrate 0 (the graze: HP loss, no wound)",
    );
}

#[test]
fn eff_pen_floors_at_zero_when_hardness_dominates() {
    let tuning = CombatTuning::default();
    let armor = armor_piece(1, 6, 50);
    let damage = 10;
    let protection = *armor.protection;
    let floor = *armor.floor;
    let result = resolve_hit(
        WeaponDamage::new(damage),
        WeaponPunch::new(5),
        WeaponShred::new(0),
        &armor,
        Matchup::Neutral,
        &tuning,
    );

    let baseline_hp = floor.max(damage - protection);
    let baseline_pen = (damage - protection).max(0);
    assert_eq!(
        *result.hp_damage, baseline_hp,
        "with effPen 0 the soak is undiminished: dmg == max(floor, damage − protection)",
    );
    assert_eq!(
        *result.penetrating, baseline_pen,
        "with effPen 0 the penetrating damage equals the no-penetration baseline",
    );
}

#[test]
fn penetrating_damage_monotonic_in_punch() {
    let tuning = CombatTuning::default();
    let armor = armor_piece(0, 10, 2);
    let resolve = |punch: i32| {
        resolve_hit(
            WeaponDamage::new(12),
            WeaponPunch::new(punch),
            WeaponShred::new(0),
            &armor,
            Matchup::Neutral,
            &tuning,
        )
    };

    let low = resolve(3);
    let high = resolve(8);
    assert!(
        *high.penetrating >= *low.penetrating,
        "higher punch must yield >= penetrating damage (got high {} < low {})",
        *high.penetrating,
        *low.penetrating,
    );
}

#[test]
fn favorable_beats_resisted_through_punch_and_shred() {
    let tuning = CombatTuning::default();
    let armor = armor_piece(1, 12, 1);
    let resolve = |matchup: Matchup| {
        resolve_hit(
            WeaponDamage::new(14),
            WeaponPunch::new(10),
            WeaponShred::new(6),
            &armor,
            matchup,
            &tuning,
        )
    };

    let favorable = resolve(Matchup::Favorable);
    let resisted = resolve(Matchup::Resisted);

    assert!(
        *favorable.penetrating >= *resisted.penetrating,
        "Favorable must yield >= penetrating damage than Resisted",
    );
    assert!(
        *favorable.wear >= *resisted.wear,
        "Favorable must yield >= integrity wear than Resisted",
    );

    let mult_fav = matchup_multiplier(Matchup::Favorable, &tuning);
    let mult_res = matchup_multiplier(Matchup::Resisted, &tuning);
    let expected_hp = |mult: MatchupMultiplier| {
        let punch_scaled = scale_by_matchup(DamageMagnitude::new(10), mult);
        let eff_pen = (*punch_scaled - *armor.hardness).max(0);
        let inner = 14 - (*armor.protection - eff_pen).max(0);
        (*armor.floor).max(inner)
    };
    assert_eq!(
        *favorable.hp_damage,
        expected_hp(mult_fav),
        "Favorable HP-loss must trace to the scaled PUNCH, not to damage/floor/protection",
    );
    assert_eq!(
        *resisted.hp_damage,
        expected_hp(mult_res),
        "Resisted HP-loss must trace to the scaled PUNCH, not to damage/floor/protection",
    );
}

#[test]
fn shred_adds_to_wear_and_matchup_leaves_damage_floor() {
    let tuning = CombatTuning::default();
    let armor = armor_piece(2, 8, 1);
    let resolve = |shred: i32, matchup: Matchup| {
        resolve_hit(
            WeaponDamage::new(9),
            WeaponPunch::new(4),
            WeaponShred::new(shred),
            &armor,
            matchup,
            &tuning,
        )
    };

    let no_shred = resolve(0, Matchup::Neutral);
    let with_shred = resolve(5, Matchup::Neutral);
    let neutral_mult = matchup_multiplier(Matchup::Neutral, &tuning);
    let scaled_shred = scale_by_matchup(DamageMagnitude::new(5), neutral_mult);
    assert_eq!(
        *with_shred.wear - *no_shred.wear,
        *scaled_shred,
        "shred must add to integrity wear on top of the soak/pen term",
    );
    assert!(
        *with_shred.wear > *no_shred.wear,
        "shred > 0 must raise integrity wear above the shred = 0 baseline",
    );

    let soaked = armor_piece(3, 30, 40);
    let soak_resolve = |matchup: Matchup| {
        resolve_hit(
            WeaponDamage::new(2),
            WeaponPunch::new(5),
            WeaponShred::new(0),
            &soaked,
            matchup,
            &tuning,
        )
    };
    let neutral = soak_resolve(Matchup::Neutral);
    let favorable = soak_resolve(Matchup::Favorable);
    assert_eq!(
        *neutral.hp_damage, *soaked.floor,
        "fully-soaked HP-loss must pin to the armor floor",
    );
    assert_eq!(
        *favorable.hp_damage, *neutral.hp_damage,
        "the matchup multiplier must NOT change damage/floor — HP-loss stays at floor",
    );
}

#[test]
fn hit_result_fields_are_named_newtypes() {
    let result = HitResult {
        penetrating: PenetratingDamage::new(7),
        hp_damage:   HpDamage::new(8),
        wear:        IntegrityWear::new(9),
    };
    assert_eq!(*result.penetrating, 7i32);
    assert_eq!(*result.hp_damage, 8i32);
    assert_eq!(*result.wear, 9i32);
}

#[test]
fn pen_and_hp_damage_split_pre_and_post_floor() {
    let tuning = CombatTuning::default();
    let armor = armor_piece(5, 25, 0);
    let result = resolve_hit(
        WeaponDamage::new(4),
        WeaponPunch::new(0),
        WeaponShred::new(0),
        &armor,
        Matchup::Neutral,
        &tuning,
    );
    assert_eq!(
        *result.hp_damage, 5,
        "HP-loss (post-floor) must be the floor bruise",
    );
    assert_eq!(
        *result.penetrating, 0,
        "penetrating (pre-floor) must be 0 — distinct from the floor-bruised HP loss",
    );
    assert_ne!(
        *result.hp_damage, *result.penetrating,
        "the two-field split: post-floor HP-loss and pre-floor penetration differ in the graze regime",
    );
}

#[test]
fn resolves_against_a_piece_from_a_source_suit() {
    let tuning = CombatTuning::default();
    let piece = armor_piece(1, 5, 2);
    let suit = SourceArmor::uniform(piece);
    let struck = suit.at(BodyPart::Torso);

    let result = resolve_hit(
        WeaponDamage::new(10),
        WeaponPunch::new(6),
        WeaponShred::new(3),
        &struck,
        Matchup::Neutral,
        &tuning,
    );

    assert_eq!(*result.hp_damage, 9, "HP-loss from the soak/pen formula");
    assert_eq!(
        *result.penetrating, 9,
        "penetrating equals inner when inner > 0"
    );
    assert_eq!(
        *result.wear, 12,
        "wear = min(protection, damage) + effPen + shred"
    );
}
