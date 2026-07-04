//! The GTW-328 combat-event classify pins — the fire / movement / shot-outcome / reload /
//! turn / move-rejection / injury arms, including the GTW-559 `None`-report guard.

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    AppliedDamage, ArmorHardness, ArmorProtection, ArmorWearOutcome, BodyPart, Cell, CellLevel,
    CoverEntry, CoverHp, CoverVerdict, Faction, GangerVerdict, HeightBand, HitReport, HitResult,
    HitVerdict, HpDamage, IntegrityWear, Level, LifeState, Matchup, ModeKind, MoveRejection,
    PenetratingDamage, PlayerFaction, ReloadOutcome, Severity, ShotKind, SlabVerdict,
};

use super::super::{
    super::{
        palette::{FctValence, severity_color, valence_color},
        text::FctEmphasis,
    },
    classify::classify_log_event,
    event::{CombatLogEvent, InjuryLogText, LogName},
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

/// A fire declaration WITH a named target reads `"<actor> fired <Mode> at <target>"`, the
/// mode title-cased, in neutral GREY.
#[test]
fn a_fire_declaration_at_a_target_names_actor_mode_and_target() {
    let event = CombatLogEvent::FireDeclaration {
        actor:  LogName::new("Vex"),
        target: Some(LogName::new("Skar")),
        mode:   ModeKind::Burst,
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1, "a fire declaration is one line");
    assert_eq!(&**lines[0].text(), "Vex fired Burst at Skar");
    assert_eq!(lines[0].color(), valence_color(FctValence::Neutral));
    assert_eq!(lines[0].emphasis(), FctEmphasis::Normal);
}

/// A fire declaration at NO named target drops the `"at <target>"` tail —
/// `"<actor> fired <Mode>"`.
#[test]
fn a_fire_declaration_at_no_target_omits_the_target_clause() {
    let event = CombatLogEvent::FireDeclaration {
        actor:  LogName::new("Vex"),
        target: None,
        mode:   ModeKind::Full,
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1);
    assert_eq!(&**lines[0].text(), "Vex fired Full");
}

/// A movement reads `"<actor> moved <from> -> <to>"` with both ground cells' coords, in
/// neutral GREY.
#[test]
fn a_movement_names_actor_and_both_cells() {
    let event = CombatLogEvent::MovementOccurred {
        actor: LogName::new("Vex"),
        from:  Cell::new(3, 4),
        to:    Cell::new(3, 6),
    };
    let lines = classify_log_event(&event);
    assert_eq!(lines.len(), 1);
    assert_eq!(&**lines[0].text(), "Vex moved (3, 4) -> (3, 6)");
    assert_eq!(lines[0].color(), valence_color(FctValence::Neutral));
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
/// blast's detonation seed rides the impact seam with a placeholder shooter + `None`
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

/// GTW-537 — a SUPPRESSED move rejection logs ONE line `"<actor> is pinned"` in wound
/// AMBER; the Unreachable / Unaffordable reasons log NOTHING (kept silent).
#[test]
fn a_suppressed_move_rejection_logs_a_pinned_line_only() {
    let suppressed = CombatLogEvent::MoveRejected {
        actor:  LogName::new("Vex"),
        reason: MoveRejection::Suppressed,
    };
    let lines = classify_log_event(&suppressed);
    assert_eq!(
        lines.len(),
        1,
        "a suppressed move rejection is exactly one log line",
    );
    assert_eq!(&**lines[0].text(), "Vex is pinned");
    assert_eq!(
        lines[0].color(),
        valence_color(FctValence::Status),
        "the pinned line is drawn in the denied-act wound amber",
    );

    // The pre-existing reasons stay SILENT (no line) — no behavior change to them.
    for reason in [MoveRejection::Unreachable, MoveRejection::Unaffordable] {
        let event = CombatLogEvent::MoveRejected {
            actor: LogName::new("Vex"),
            reason,
        };
        assert!(
            classify_log_event(&event).is_empty(),
            "{reason:?} must stay unsurfaced (no log line) — pre-GTW-537 behavior preserved",
        );
    }
}

/// A successful reload reads `"<actor> reloaded"` (neutral); a no-TU reload reads
/// `"<actor>: no TU"` (AMBER); an already-full reload yields NO line.
#[test]
fn the_reload_outcomes_each_phrase_distinctly() {
    let reloaded = CombatLogEvent::ReloadResult {
        actor:   LogName::new("Vex"),
        outcome: ReloadOutcome::Reloaded,
    };
    let lines = classify_log_event(&reloaded);
    assert_eq!(lines.len(), 1);
    assert_eq!(&**lines[0].text(), "Vex reloaded");
    assert_eq!(lines[0].color(), valence_color(FctValence::Neutral));

    let no_tu = CombatLogEvent::ReloadResult {
        actor:   LogName::new("Vex"),
        outcome: ReloadOutcome::NoTu,
    };
    let no_tu_lines = classify_log_event(&no_tu);
    assert_eq!(no_tu_lines.len(), 1);
    assert_eq!(&**no_tu_lines[0].text(), "Vex: no TU");
    assert_eq!(no_tu_lines[0].color(), valence_color(FctValence::Status));

    let full = CombatLogEvent::ReloadResult {
        actor:   LogName::new("Vex"),
        outcome: ReloadOutcome::AlreadyFull,
    };
    assert!(
        classify_log_event(&full).is_empty(),
        "an already-full reload must yield no log line",
    );
}

/// A turn boundary reads `"— Player turn —"` when the newly-active gang is the player's,
/// else `"— Enemy turn —"`. Neutral GREY.
#[test]
fn a_turn_boundary_labels_player_vs_enemy_by_faction() {
    let player = PlayerFaction::new(Faction::new(0));

    let player_turn = CombatLogEvent::TurnStarted {
        now_active: Faction::new(0),
        player,
    };
    let lines = classify_log_event(&player_turn);
    assert_eq!(lines.len(), 1);
    assert_eq!(&**lines[0].text(), "— Player turn —");
    assert_eq!(lines[0].color(), valence_color(FctValence::Neutral));

    let enemy_turn = CombatLogEvent::TurnStarted {
        now_active: Faction::new(1),
        player,
    };
    let enemy_lines = classify_log_event(&enemy_turn);
    assert_eq!(enemy_lines.len(), 1);
    assert_eq!(&**enemy_lines[0].text(), "— Enemy turn —");
}

/// GTW-439 — an inflicted injury logs ONE line `"<actor> <log_text>"`, drawn in the
/// severity-scaled wound AMBER and body weight (not bold).
#[test]
fn an_injury_logs_the_actor_and_authored_clause_in_the_severity_color() {
    let event = CombatLogEvent::InjuryInflicted {
        actor:    LogName::new("Vex"),
        log_text: InjuryLogText::new("loses an eye"),
        severity: Severity::Critical,
    };
    let lines = classify_log_event(&event);
    assert_eq!(
        lines.len(),
        1,
        "an inflicted injury is exactly one log line"
    );
    assert_eq!(&**lines[0].text(), "Vex loses an eye");
    assert_eq!(
        lines[0].color(),
        severity_color(Severity::Critical),
        "the injury line is drawn the Critical-scaled wound amber",
    );
    assert_eq!(lines[0].emphasis(), FctEmphasis::Normal);
}

/// GTW-439 — PIN-DISCRIMINATING: the injury line's color TRACKS the rolled tier — a Minor
/// injury reads a DIFFERENT swatch than a Critical one, so a flat-valence routing fails.
#[test]
fn the_injury_log_color_scales_with_severity() {
    let minor = CombatLogEvent::InjuryInflicted {
        actor:    LogName::new("Vex"),
        log_text: InjuryLogText::new("twists an ankle"),
        severity: Severity::Minor,
    };
    let critical = CombatLogEvent::InjuryInflicted {
        actor:    LogName::new("Vex"),
        log_text: InjuryLogText::new("loses an eye"),
        severity: Severity::Critical,
    };
    let minor_color = classify_log_event(&minor)[0].color();
    let critical_color = classify_log_event(&critical)[0].color();
    assert_eq!(minor_color, severity_color(Severity::Minor));
    assert_eq!(critical_color, severity_color(Severity::Critical));
    assert_ne!(
        minor_color, critical_color,
        "a Critical injury must log a hotter swatch than a Minor one (color by severity)",
    );
}
