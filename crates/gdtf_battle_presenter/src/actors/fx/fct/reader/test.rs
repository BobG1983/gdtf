//! Unit tests for the [`HitReport`] → pops classification surface (moved whole from
//! the old `reader.rs` inline `mod test`).

use bevy::ecs::entity::Entity;
use gdtf_battle_sim::{
    AppliedDamage, ArmorHardness, ArmorProtection, ArmorWearOutcome, BodyPart, Cell, CellLevel,
    CoverEntry, CoverHp, CoverVerdict, GangerVerdict, HeightBand, HitReport, HitResult, HitVerdict,
    HpDamage, IntegrityWear, Level, LifeState, Matchup, PenetratingDamage, Severity, ShotKind,
    SlabVerdict,
};

use super::{
    super::{
        palette::{FctValence, severity_color, valence_color},
        text::FctEmphasis,
    },
    classified::ClassifiedPop,
    classify::classify_report,
};

/// An arbitrary `(cell, level)` key for a structural-hit report (the classifier reads the
/// verdict / the destruction flag, never the key's coords, so any value drives the path).
fn struck_key() -> CellLevel {
    CellLevel::new(Cell::new(4, 5), Level::new(2))
}

/// An arbitrary intact `CoverEntry` for a `ShotKind::Cover` outcome — the classifier only
/// matches the verdict, never reads the entry's HP, so a seeded prototype is enough.
fn cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    )
}

/// A cover report that DAMAGED but did not destroy the cover (a REAL cover verdict
/// with `destroyed: None` — GTW-573) — the structural-hit (not-destroyed) path.
fn cover_damaged_report() -> HitReport {
    HitReport {
        kind:    ShotKind::Cover(cover_entry()),
        verdict: HitVerdict::Cover(CoverVerdict { destroyed: None }),
    }
}

/// A cover report that DESTROYED the cover (the verdict's `destroyed` is `Some`) — the
/// structural-destruction path.
fn cover_destroyed_report() -> HitReport {
    HitReport {
        kind:    ShotKind::Cover(cover_entry()),
        verdict: HitVerdict::Cover(CoverVerdict {
            destroyed: Some(struck_key()),
        }),
    }
}

/// A slab report that DESTROYED the slab (the verdict's `destroyed` is `Some`).
fn slab_destroyed_report() -> HitReport {
    HitReport {
        kind:    ShotKind::Slab(struck_key()),
        verdict: HitVerdict::Slab(SlabVerdict {
            destroyed: Some(struck_key()),
        }),
    }
}

/// A `HitResult` carrying `hp` HP loss + `pen` penetrating damage (zero wear — the wear
/// number is not an FCT input this slice).
fn hit_result(hp: i32, pen: i32) -> HitResult {
    HitResult {
        penetrating: PenetratingDamage::new(pen),
        hp_damage:   HpDamage::new(hp),
        wear:        IntegrityWear::new(0),
    }
}

/// A ganger-hit `HitReport` for `part` with `hp` HP loss / `pen` penetration / `severity`
/// tier / `life_after` state — the synthesized report the classifier reads.
fn ganger_report(
    part: BodyPart,
    hp: i32,
    pen: i32,
    severity: Severity,
    life_after: LifeState,
) -> HitReport {
    HitReport {
        // The classifier only matches on the Ganger verdict — it never dereferences the
        // entity — so a placeholder handle is enough to drive the Ganger branch.
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

/// The set of `(text, color)` pairs a report classifies into, for order-insensitive
/// membership asserts.
fn pop_pairs(report: Option<&HitReport>) -> Vec<(String, bevy::prelude::Color)> {
    classify_report(report)
        .into_iter()
        .map(|ClassifiedPop { text, color, .. }| ((*text).clone(), color))
        .collect()
}

/// Whether the classified pops contain a pop with exactly `text` in exactly `color`.
fn has_pop(report: Option<&HitReport>, text: &str, color: bevy::prelude::Color) -> bool {
    pop_pairs(report)
        .iter()
        .any(|(t, c)| t == text && *c == color)
}

/// The [`FctEmphasis`] of the classified pop with exactly `text`, or `None` if absent.
fn pop_emphasis(report: Option<&HitReport>, text: &str) -> Option<FctEmphasis> {
    classify_report(report)
        .into_iter()
        .find(|pop| *pop.text == *text)
        .map(|pop| pop.emphasis)
}

/// A damaging ganger hit yields the RED HP-loss number `-N`.
#[test]
fn a_damage_hit_yields_the_red_hp_number() {
    let report = ganger_report(BodyPart::Torso, 7, 5, Severity::Minor, LifeState::Alive);
    assert!(
        has_pop(Some(&report), "-7", valence_color(FctValence::Damage)),
        "a 7-HP hit must yield a RED \"-7\" pop, got {:?}",
        pop_pairs(Some(&report)),
    );
}

/// A wounding hit yields the `"<Part> <Tier>"` pop in the severity AMBER ramp (and the L/R
/// limb collapses to the side-less label).
#[test]
fn a_wound_yields_the_part_tier_pop_in_the_severity_color() {
    let report = ganger_report(BodyPart::LeftArm, 4, 3, Severity::Major, LifeState::Alive);
    assert!(
        has_pop(Some(&report), "Arm Major", severity_color(Severity::Major)),
        "a Major left-arm wound must yield \"Arm Major\" in the Major amber, got {:?}",
        pop_pairs(Some(&report)),
    );
}

/// A graze (severity `None`) yields the GREY `"Grazed"` pop (HP loss, no Wound), NOT a wound
/// tag.
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

/// A clean miss — a non-connecting shot (or a `None` report) — yields NO pops at all: a
/// missed shot spawns no floating combat text.
#[test]
fn a_clean_miss_yields_no_pops() {
    let miss = HitReport::no_effect(ShotKind::Miss);
    let pairs = pop_pairs(Some(&miss));
    assert!(
        pairs.is_empty(),
        "a clean miss must yield no pops, got {pairs:?}"
    );
    // A round with NO report at all (geometry-only message) is likewise a clean miss.
    let none_pairs = pop_pairs(None);
    assert!(
        none_pairs.is_empty(),
        "a None report must yield no pops, got {none_pairs:?}"
    );
}

/// A hit that DOWNS the target yields the lethal-RED `"DOWN"` pop; a hit that KILLS yields
/// `"DEAD"`. Both are drawn [`FctEmphasis::Bold`] (the contract's "RED bold"), distinct from
/// the ordinary body-weight HP-damage pop. An Alive outcome yields neither.
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
    // The same round's ordinary HP-damage pop stays body weight — bold is reserved for the
    // lethal tag, so the descope to all-caps-only is no longer the emphasis.
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

    // An Alive outcome yields no DOWN/DEAD tag.
    let alive = ganger_report(BodyPart::Torso, 3, 2, Severity::Minor, LifeState::Alive);
    let alive_pairs = pop_pairs(Some(&alive));
    assert!(
        !alive_pairs.iter().any(|(t, _)| t == "DOWN" || t == "DEAD"),
        "an Alive outcome must yield no DOWN/DEAD tag, got {alive_pairs:?}",
    );
}

/// The penetration verdict pin-discriminates: a penetrating hit (`pen > 0`) yields the GREY
/// `"Armor pierced"` pop; a soaked hit (`pen <= 0`) yields the AMBER `"Armor held"` pop.
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

/// GTW-386 — a cover hit that DAMAGED (did not destroy) the cover yields a NON-EMPTY
/// structural pop list (the `"Cover hit"` chip indicator in neutral GREY), NOT the empty /
/// miss fall-through. PIN-DISCRIMINATING: reverting the classifier's `ShotKind::Cover` branch
/// (back to the old "non-ganger → empty vec" gate) makes this fail (the list would be empty).
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

/// GTW-386 — a cover hit that DESTROYED the cover (`cover_destroyed: Some`) yields the
/// emphatic lethal-RED BOLD `"Cover Destroyed"` pop (the structural mirror of a ganger's
/// DOWN / DEAD tag), NOT a plain hit or a miss. PIN-DISCRIMINATING on both the branch and the
/// destruction-flag read.
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

/// GTW-386 — a SLAB hit that DESTROYED the slab (`slab_destroyed: Some`) yields the lethal-RED
/// BOLD `"Slab Destroyed"` pop, NOT a miss — the slab mirror of the cover-destroyed case.
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

/// GTW-386 — a GROUND hit yields the cosmetic neutral-GREY `"Dust"` impact cue (a minor
/// indicator, never a damage number — the ground is damaged, never destroyed), NOT a miss.
#[test]
fn a_ground_hit_yields_the_dust_cue_not_a_miss() {
    // The GTW-573 ground verdict — the sim's Ground arm ALWAYS accrues (an arbitrary
    // amount here; the classifier reads only the variant).
    let report = HitReport {
        kind:    ShotKind::Ground(struck_key()),
        verdict: gdtf_battle_sim::HitVerdict::Ground(gdtf_battle_sim::GroundAccrual::new(
            Cell::new(4, 5),
            gdtf_battle_sim::GroundDamage::new(3),
        )),
    };
    let pairs = pop_pairs(Some(&report));
    assert!(
        has_pop(Some(&report), "Dust", valence_color(FctValence::Neutral)),
        "a ground hit must yield a GREY \"Dust\" cosmetic cue, got {pairs:?}",
    );
}
