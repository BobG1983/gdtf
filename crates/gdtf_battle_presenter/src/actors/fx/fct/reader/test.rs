use bevy::ecs::entity::Entity;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection, BodyPart},
    armor_wear::ArmorWearOutcome,
    cover::{CoverEntry, CoverHp, HeightBand},
    entity::TerrainPieceKind,
    matchup::Matchup,
    prelude::{Cell, CellLevel, Level, LifeState},
    resolve_and_apply::{
        AppliedDamage, CoverVerdict, GangerVerdict, HitReport, HitVerdict, SlabVerdict,
    },
    resolve_coarse::ShotKind,
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    severity::Severity,
};

use super::{
    super::{
        palette::{FctValence, severity_color, valence_color},
        text::FctEmphasis,
    },
    classified::ClassifiedPop,
    classify::classify_report,
};

fn struck_key() -> CellLevel {
    CellLevel::new(Cell::new(4, 5), Level::new(2))
}

fn cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        TerrainPieceKind::Cover,
    )
}

fn cover_damaged_report() -> HitReport {
    HitReport {
        kind:    ShotKind::Cover(cover_entry()),
        verdict: HitVerdict::Cover(CoverVerdict {
            destroyed: None,
            kind:      TerrainPieceKind::Cover,
        }),
    }
}

fn cover_destroyed_report() -> HitReport {
    HitReport {
        kind:    ShotKind::Cover(cover_entry()),
        verdict: HitVerdict::Cover(CoverVerdict {
            destroyed: Some(struck_key()),
            kind:      TerrainPieceKind::Cover,
        }),
    }
}

fn slab_destroyed_report() -> HitReport {
    HitReport {
        kind:    ShotKind::Slab(struck_key()),
        verdict: HitVerdict::Slab(SlabVerdict {
            destroyed: Some(struck_key()),
        }),
    }
}

fn hit_result(hp: i32, pen: i32) -> HitResult {
    HitResult {
        penetrating: PenetratingDamage::new(pen),
        hp_damage:   HpDamage::new(hp),
        wear:        IntegrityWear::new(0),
    }
}

fn ganger_report(
    part: BodyPart,
    hp: i32,
    pen: i32,
    severity: Severity,
    life_after: LifeState,
) -> HitReport {
    HitReport {
        kind:    ShotKind::Ganger(Entity::PLACEHOLDER),
        verdict: HitVerdict::Ganger(Box::new(GangerVerdict {
            target: Entity::PLACEHOLDER,
            part,
            applied: AppliedDamage {
                matchup: Matchup::Neutral,
                hit: hit_result(hp, pen),
                severity,
                life_after,
                wear: ArmorWearOutcome::Unaffected,
            },
            injury: None,
            dot_applied: None,
        })),
    }
}

fn pop_pairs(report: Option<&HitReport>) -> Vec<(String, bevy::prelude::Color)> {
    classify_report(report)
        .into_iter()
        .map(|ClassifiedPop { text, color, .. }| ((*text).clone(), color))
        .collect()
}

fn has_pop(report: Option<&HitReport>, text: &str, color: bevy::prelude::Color) -> bool {
    pop_pairs(report)
        .iter()
        .any(|(t, c)| t == text && *c == color)
}

fn pop_emphasis(report: Option<&HitReport>, text: &str) -> Option<FctEmphasis> {
    classify_report(report)
        .into_iter()
        .find(|pop| *pop.text == *text)
        .map(|pop| pop.emphasis)
}

#[test]
fn a_damage_hit_yields_the_red_hp_number() {
    let report = ganger_report(BodyPart::Torso, 7, 5, Severity::Minor, LifeState::Alive);
    assert!(
        has_pop(Some(&report), "-7", valence_color(FctValence::Damage)),
        "a 7-HP hit must yield a RED \"-7\" pop, got {:?}",
        pop_pairs(Some(&report)),
    );
}

#[test]
fn a_wound_yields_the_part_tier_pop_in_the_severity_color() {
    let report = ganger_report(BodyPart::LeftArm, 4, 3, Severity::Major, LifeState::Alive);
    assert!(
        has_pop(Some(&report), "Arm Major", severity_color(Severity::Major)),
        "a Major left-arm wound must yield \"Arm Major\" in the Major amber, got {:?}",
        pop_pairs(Some(&report)),
    );
}

#[test]
fn a_graze_yields_grey_grazed_not_a_wound() {
    let report = ganger_report(BodyPart::Torso, 2, 0, Severity::None, LifeState::Alive);
    let pairs = pop_pairs(Some(&report));
    assert!(
        has_pop(Some(&report), "Grazed", valence_color(FctValence::Neutral)),
        "a Severity::None hit must yield a GREY \"Grazed\" pop, got {pairs:?}",
    );
    assert!(
        !pairs.iter().any(|(t, _)| t.contains("Torso")),
        "a graze must NOT yield a body-part wound tag, got {pairs:?}",
    );
}

#[test]
fn a_clean_miss_yields_no_pops() {
    let miss = HitReport::no_effect(ShotKind::Miss);
    let pairs = pop_pairs(Some(&miss));
    assert!(
        pairs.is_empty(),
        "a clean miss must yield no pops, got {pairs:?}"
    );
    let none_pairs = pop_pairs(None);
    assert!(
        none_pairs.is_empty(),
        "a None report must yield no pops, got {none_pairs:?}"
    );
}

#[test]
fn a_down_and_a_dead_yield_the_lethal_red_bold_tag() {
    let down = ganger_report(BodyPart::Torso, 9, 6, Severity::Critical, LifeState::Downed);
    assert!(
        has_pop(Some(&down), "DOWN", valence_color(FctValence::Lethal)),
        "a Downed outcome must yield a lethal-RED \"DOWN\" pop, got {:?}",
        pop_pairs(Some(&down)),
    );
    assert_eq!(
        pop_emphasis(Some(&down), "DOWN"),
        Some(FctEmphasis::Bold),
        "the \"DOWN\" pop must be BOLD (the contract's \"RED bold\"), not body weight",
    );
    assert_eq!(
        pop_emphasis(Some(&down), "-9"),
        Some(FctEmphasis::Normal),
        "an ordinary HP-damage pop must stay body weight, not bold",
    );

    let dead = ganger_report(BodyPart::Head, 12, 10, Severity::Fatal, LifeState::Dead);
    assert!(
        has_pop(Some(&dead), "DEAD", valence_color(FctValence::Lethal)),
        "a Dead outcome must yield a lethal-RED \"DEAD\" pop, got {:?}",
        pop_pairs(Some(&dead)),
    );
    assert_eq!(
        pop_emphasis(Some(&dead), "DEAD"),
        Some(FctEmphasis::Bold),
        "the \"DEAD\" pop must be BOLD (the contract's \"RED bold\"), not body weight",
    );

    let alive = ganger_report(BodyPart::Torso, 3, 2, Severity::Minor, LifeState::Alive);
    let alive_pairs = pop_pairs(Some(&alive));
    assert!(
        !alive_pairs.iter().any(|(t, _)| t == "DOWN" || t == "DEAD"),
        "an Alive outcome must yield no DOWN/DEAD tag, got {alive_pairs:?}",
    );
}

#[test]
fn the_penetration_verdict_discriminates_armor_pierced_from_armor_held() {
    let through = ganger_report(BodyPart::Torso, 5, 4, Severity::Minor, LifeState::Alive);
    assert!(
        has_pop(
            Some(&through),
            "Armor pierced",
            valence_color(FctValence::Neutral)
        ),
        "pen > 0 must yield a GREY \"Armor pierced\" pop, got {:?}",
        pop_pairs(Some(&through)),
    );

    let soaked = ganger_report(BodyPart::Torso, 1, 0, Severity::None, LifeState::Alive);
    assert!(
        has_pop(
            Some(&soaked),
            "Armor held",
            valence_color(FctValence::Status)
        ),
        "pen == 0 must yield an AMBER \"Armor held\" pop, got {:?}",
        pop_pairs(Some(&soaked)),
    );
}

#[test]
fn a_cover_damage_hit_yields_a_structural_pop_not_a_miss() {
    let report = cover_damaged_report();
    let pairs = pop_pairs(Some(&report));
    assert!(
        !pairs.is_empty(),
        "a cover hit must yield a NON-EMPTY structural pop list (not the miss \
         fall-through), got {pairs:?}",
    );
    assert!(
        has_pop(
            Some(&report),
            "Cover hit",
            valence_color(FctValence::Neutral)
        ),
        "a non-destroying cover hit must yield a GREY \"Cover hit\" chip pop, got {pairs:?}",
    );
}

#[test]
fn a_cover_destroyed_hit_yields_the_bold_destroyed_pop() {
    let report = cover_destroyed_report();
    let pairs = pop_pairs(Some(&report));
    assert!(
        has_pop(
            Some(&report),
            "Cover Destroyed",
            valence_color(FctValence::Lethal)
        ),
        "a destroying cover hit must yield a lethal-RED \"Cover Destroyed\" pop, got {pairs:?}",
    );
    assert_eq!(
        pop_emphasis(Some(&report), "Cover Destroyed"),
        Some(FctEmphasis::Bold),
        "the \"Cover Destroyed\" pop must be BOLD (the structural mirror of DOWN / DEAD)",
    );
}

#[test]
fn a_slab_destroyed_hit_yields_the_bold_destroyed_pop() {
    let report = slab_destroyed_report();
    let pairs = pop_pairs(Some(&report));
    assert!(
        !pairs.is_empty(),
        "a slab hit must yield a NON-EMPTY structural pop list, got {pairs:?}",
    );
    assert!(
        has_pop(
            Some(&report),
            "Slab Destroyed",
            valence_color(FctValence::Lethal)
        ),
        "a destroying slab hit must yield a lethal-RED \"Slab Destroyed\" pop, got {pairs:?}",
    );
    assert_eq!(
        pop_emphasis(Some(&report), "Slab Destroyed"),
        Some(FctEmphasis::Bold),
        "the \"Slab Destroyed\" pop must be BOLD",
    );
}

#[test]
fn a_ground_hit_yields_the_dust_cue_not_a_miss() {
    let report = HitReport {
        kind:    ShotKind::Ground(struck_key()),
        verdict: gdtf_battle_sim::resolve_and_apply::HitVerdict::Ground(
            gdtf_battle_sim::resolve_and_apply::GroundAccrual::new(
                Cell::new(4, 5),
                gdtf_battle_sim::surface::GroundDamage::new(3),
            ),
        ),
    };
    let pairs = pop_pairs(Some(&report));
    assert!(
        has_pop(Some(&report), "Dust", valence_color(FctValence::Neutral)),
        "a ground hit must yield a GREY \"Dust\" cosmetic cue, got {pairs:?}",
    );
}
