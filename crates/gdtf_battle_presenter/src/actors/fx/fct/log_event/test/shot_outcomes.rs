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

fn struck_key() -> CellLevel {
    CellLevel::new(Cell::new(4, 5), Level::new(2))
}

fn cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    )
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

fn line_pairs(event: &CombatLogEvent) -> Vec<(String, bevy::prelude::Color)> {
    classify_log_event(event)
        .into_iter()
        .map(|line| ((**line.text()).clone(), line.color()))
        .collect()
}

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
    let lines = classify_log_event(&event);
    let down = lines.iter().find(|line| &***line.text() == "DOWN");
    assert!(
        down.is_some_and(|line| line.color() == valence_color(FctValence::Lethal)
            && line.emphasis() == FctEmphasis::Bold),
        "a downing hit must yield a lethal-RED BOLD \"DOWN\" line, got {pairs:?}",
    );
}

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

#[test]
fn a_cover_hit_logs_a_structural_line_not_a_miss() {
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
