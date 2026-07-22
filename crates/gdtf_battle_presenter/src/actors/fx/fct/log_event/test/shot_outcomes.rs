//! The GTW-328 shot-outcome classify pins — the `classify_report` reuse, the explicit
//! miss line, the GTW-559 `None`-report guard, and the GTW-386 structural cover / slab
//! destroyed lines.

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection, BodyPart},
    armor_wear::ArmorWearOutcome,
    cover::{CoverEntry, CoverHp, HeightBand},
    matchup::Matchup,
    prelude::{Cell, CellLevel, Level, LifeState},
    resolve_and_apply::{
        AppliedDamage, CoverVerdict, GangerVerdict, HitReport, HitVerdict, SlabVerdict,
    },
    resolve_coarse::ShotKind,
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    severity::Severity,
};

use super::super::{
    super::{
        palette::{FctValence, valence_color},
        text::FctEmphasis,
    },
    classify::classify_log_event,
    event::{CombatLogEvent, LogName},
};

/// An arbitrary `(cell, level)` key for a structural-hit report.
fn struck_key() -> CellLevel {
    CellLevel::new(Cell::new(4, 5), Level::new(2))
}

/// An arbitrary intact `CoverEntry` for a `ShotKind::Cover` outcome.
fn cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    )
}

/// A ganger-hit `HitReport` for `part` with `hp` HP loss / `pen` penetration / `severity`
/// tier / `life_after` state — the synthesized report a `ShotOutcome` reads.
fn ganger_report(
    part: BodyPart,
    hp: i32,
    pen: i32,
    severity: Severity,
    life_after: LifeState,
) -> HitReport {
    HitReport {
        // classify_report only matches the Ganger verdict, never derefs the entity.
        kind:    ShotKind::Ganger(Entity::PLACEHOLDER),
        verdict: HitVerdict::Ganger(Box::new(GangerVerdict {
            target: Entity::PLACEHOLDER,
            part,
            applied: AppliedDamage {
                matchup: Matchup::Neutral,
                hit: HitResult {
                    penetrating: PenetratingDamage::new(pen),
                    hp_damage:   HpDamage::new(hp),
                    wear:        IntegrityWear::new(0),
                },
                severity,
                life_after,
                wear: ArmorWearOutcome::Unaffected,
            },
            injury: None,
            dot_applied: None,
        })),
    }
}

/// The `(text, color)` pairs a classified event yields, for membership asserts.
fn line_pairs(event: &CombatLogEvent) -> Vec<(String, bevy::prelude::Color)> {
    classify_log_event(event)
        .into_iter()
        .map(|line| ((**line.text()).clone(), line.color()))
        .collect()
}

/// A CONNECTING shot outcome reuses `classify_report`: a damaging, downing hit yields the
/// RED HP-loss line AND the lethal BOLD `"DOWN"` line (no name prefix — the report lines
/// come straight from the shared classifier).
#[test]
fn a_connecting_shot_outcome_reuses_classify_report_lines() {
    let report = ganger_report(BodyPart::Torso, 9, 6, Severity::Critical, LifeState::Downed);
    let event = CombatLogEvent::ShotOutcome {
        actor:  LogName::new("Vex"),
        report: Some(Box::new(report)),
    };
    let pairs = line_pairs(&event);
    assert!(
        pairs
            .iter()
            .any(|(t, c)| t == "-9" && *c == valence_color(FctValence::Damage)),
        "a 9-HP hit must yield a RED \"-9\" line (from classify_report), got {pairs:?}",
    );
    // The DOWN line is the lethal-RED BOLD line — emphasis carried through from the pop.
    let lines = classify_log_event(&event);
    let down = lines.iter().find(|line| &***line.text() == "DOWN");
    assert!(
        down.is_some_and(|line| line.color() == valence_color(FctValence::Lethal)
            && line.emphasis() == FctEmphasis::Bold),
        "a downing hit must yield a lethal-RED BOLD \"DOWN\" line, got {pairs:?}",
    );
}

/// A clean MISS is NEVER suppressed — it reads `"<actor> missed"` in neutral GREY (the user
/// explicitly wants misses logged, unlike the floating-combat-text which drops them).
#[test]
fn a_clean_miss_yields_the_explicit_missed_line() {
    let event = CombatLogEvent::ShotOutcome {
        actor:  LogName::new("Vex"),
        report: Some(Box::new(HitReport::no_effect(ShotKind::Miss))),
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1, "a clean miss is exactly one log line");
    assert_eq!(&**lines[0].text(), "Vex missed");
    assert_eq!(lines[0].color(), valence_color(FctValence::Neutral));
}

/// GTW-559 — a `None` report is NOT a miss: it carries no ganger-shot verdict (the grenade
/// blast's detonation seed rides the impact handoff with a placeholder shooter + `None`
/// report), so it yields NO log line at all. PIN-DISCRIMINATING: the old classifier rendered
/// it as `"<actor> missed"` — the phantom `"Someone missed"` every detonation appended.
/// A REAL clean miss keeps its line (it always carries `Some(HitReport)` with
/// `ShotKind::Miss` — pinned by [`a_clean_miss_yields_the_explicit_missed_line`] above).
#[test]
fn a_verdict_less_none_report_yields_no_line_not_a_phantom_miss() {
    let none_event = CombatLogEvent::ShotOutcome {
        actor:  LogName::new("Someone"),
        report: None,
    };
    assert!(
        classify_log_event(&none_event).is_empty(),
        "a None-report shot outcome (a blast detonation — no ganger-shot verdict) must \
         yield NO log line, never a phantom miss",
    );
}

/// GTW-386 — a COVER hit logs a real structural line, NOT `"<actor> missed"`: a damaging hit
/// reads the `"Cover hit"` chip line, and a DESTROYING hit reads the lethal-RED BOLD
/// `"Cover Destroyed"` line.
#[test]
fn a_cover_hit_logs_a_structural_line_not_a_miss() {
    // Damaged (not destroyed) — a "Cover hit" chip line, never the miss line.
    let damaged = CombatLogEvent::ShotOutcome {
        actor:  LogName::new("Vex"),
        report: Some(Box::new(HitReport {
            kind:    ShotKind::Cover(cover_entry()),
            verdict: HitVerdict::Cover(CoverVerdict { destroyed: None }),
        })),
    };
    let lines = classify_log_event(&damaged);
    assert!(
        lines.iter().all(|line| !(**line.text()).contains("missed")),
        "a cover hit must NOT log a \"missed\" line, got {:?}",
        line_pairs(&damaged),
    );
    assert!(
        lines.iter().any(|line| &***line.text() == "Cover hit"
            && line.color() == valence_color(FctValence::Neutral)),
        "a damaging cover hit must log a GREY \"Cover hit\" line, got {:?}",
        line_pairs(&damaged),
    );

    // Destroyed — the emphatic lethal-RED BOLD "Cover Destroyed" line.
    let destroyed_report = HitReport {
        kind:    ShotKind::Cover(cover_entry()),
        verdict: HitVerdict::Cover(CoverVerdict {
            destroyed: Some(struck_key()),
        }),
    };
    let destroyed = CombatLogEvent::ShotOutcome {
        actor:  LogName::new("Vex"),
        report: Some(Box::new(destroyed_report)),
    };
    let destroyed_lines = classify_log_event(&destroyed);
    let line = destroyed_lines
        .iter()
        .find(|line| &***line.text() == "Cover Destroyed");
    assert!(
        line.is_some_and(|line| line.color() == valence_color(FctValence::Lethal)
            && line.emphasis() == FctEmphasis::Bold),
        "a destroying cover hit must log a lethal-RED BOLD \"Cover Destroyed\" line, got {:?}",
        line_pairs(&destroyed),
    );
}

/// GTW-386 — a SLAB hit that DESTROYED the slab logs the lethal-RED BOLD `"Slab Destroyed"`
/// line, never `"<actor> missed"` (the slab mirror of the cover-destroyed log case).
#[test]
fn a_slab_destroyed_hit_logs_the_destroyed_line_not_a_miss() {
    let report = HitReport {
        kind:    ShotKind::Slab(struck_key()),
        verdict: HitVerdict::Slab(SlabVerdict {
            destroyed: Some(struck_key()),
        }),
    };
    let event = CombatLogEvent::ShotOutcome {
        actor:  LogName::new("Vex"),
        report: Some(Box::new(report)),
    };
    let lines = classify_log_event(&event);
    assert!(
        lines.iter().all(|line| !(**line.text()).contains("missed")),
        "a slab hit must NOT log a \"missed\" line, got {:?}",
        line_pairs(&event),
    );
    let line = lines
        .iter()
        .find(|line| &***line.text() == "Slab Destroyed");
    assert!(
        line.is_some_and(|line| line.color() == valence_color(FctValence::Lethal)
            && line.emphasis() == FctEmphasis::Bold),
        "a destroying slab hit must log a lethal-RED BOLD \"Slab Destroyed\" line, got {:?}",
        line_pairs(&event),
    );
}
