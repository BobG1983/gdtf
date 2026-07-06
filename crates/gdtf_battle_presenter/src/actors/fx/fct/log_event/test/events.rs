//! The GTW-328 combat-event classify pins — the fire / movement / reload / turn /
//! move-rejection / injury arms (the one-line classifications; the shot-outcome arms
//! live in [`super::shot_outcomes`]).

use gdtf_battle_sim::{
    acts::{MoveRejection, ReloadOutcome},
    battle::PlayerFaction,
    prelude::{Cell, Faction},
    severity::Severity,
    weapon::ModeKind,
};

use super::super::{
    super::{
        palette::{FctValence, severity_color, valence_color},
        text::FctEmphasis,
    },
    classify::classify_log_event,
    event::{CombatLogEvent, InjuryLogText, LogName},
};

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
