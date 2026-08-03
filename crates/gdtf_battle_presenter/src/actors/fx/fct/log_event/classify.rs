//! Map combat log events to rendered log lines.

use gdtf_battle_sim::{
    acts::{MoveRejection, ReloadOutcome},
    battle::PlayerFaction,
    falls::StoreysFallen,
    prelude::{Cell, Faction},
    resolve_and_apply::HitReport,
    resolve_hit::HpDamage,
    severity::Severity,
    weapon::{DotDamage, ModeKind},
};

use super::{
    super::{
        palette::{FctValence, severity_color, valence_color},
        reader::classify_report,
        text::{CombatText, FctEmphasis},
    },
    event::{CombatLogEvent, InjuryLogText, LogName},
    line::LogLine,
};

/// Turn one combat log event into zero or more display lines.
#[must_use]
pub fn classify_log_event(event: &CombatLogEvent) -> Vec<LogLine> {
    match event {
        CombatLogEvent::FireDeclaration {
            actor,
            target,
            mode,
        } => vec![fire_declaration_line(actor, target.as_ref(), *mode)],
        CombatLogEvent::MovementOccurred { actor, from, to } => {
            vec![movement_line(actor, *from, *to)]
        }
        CombatLogEvent::ShotOutcome { actor, report } => {
            shot_outcome_lines(actor, report.as_deref())
        }
        CombatLogEvent::ReloadResult { actor, outcome } => reload_lines(actor, *outcome),
        CombatLogEvent::TurnStarted { now_active, player } => {
            vec![turn_line(*now_active, *player)]
        }
        CombatLogEvent::MoveRejected { actor, reason } => move_rejected_lines(actor, *reason),
        CombatLogEvent::InjuryInflicted {
            actor,
            log_text,
            severity,
        } => vec![injury_line(actor, log_text, *severity)],
        CombatLogEvent::FallOccurred { actor, storeys } => vec![fall_line(actor, *storeys)],
        CombatLogEvent::MeleeStruck {
            attacker,
            target,
            amount,
        } => vec![melee_struck_line(attacker, target, *amount)],
        CombatLogEvent::OnDeathOccurred { actor } => vec![death_line(actor)],
        CombatLogEvent::SuppressionApplied { actor } => vec![suppression_line(actor)],
        CombatLogEvent::ArmorBroken { actor } => vec![armor_broken_line(actor)],
        CombatLogEvent::DotAfflicted { actor, per_turn } => {
            vec![dot_afflicted_line(actor, *per_turn)]
        }
        CombatLogEvent::FieldAfflicted { actor } => vec![field_afflicted_line(actor)],
        CombatLogEvent::BleedStarted { actor } => vec![bleed_started_line(actor)],
    }
}

fn move_rejected_lines(actor: &LogName, reason: MoveRejection) -> Vec<LogLine> {
    match reason {
        MoveRejection::Suppressed => vec![LogLine::new(
            CombatText::new(format!("{} is pinned", **actor)),
            valence_color(FctValence::Status),
        )],
        MoveRejection::Unreachable | MoveRejection::Unaffordable => Vec::new(),
    }
}

fn injury_line(actor: &LogName, log_text: &InjuryLogText, severity: Severity) -> LogLine {
    let text = format!("{} {}", **actor, **log_text);
    LogLine::new(CombatText::new(text), severity_color(severity))
}

fn fall_line(actor: &LogName, storeys: StoreysFallen) -> LogLine {
    let count = *storeys;
    let noun = if count == 1 { "storey" } else { "storeys" };
    LogLine::new(
        CombatText::new(format!("{} fell {count} {noun}", **actor)),
        valence_color(FctValence::Status),
    )
}

fn melee_struck_line(attacker: &LogName, target: &LogName, amount: HpDamage) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} struck {} (-{})", **attacker, **target, *amount)),
        valence_color(FctValence::Damage),
    )
}

fn death_line(actor: &LogName) -> LogLine {
    LogLine::new_bold(
        CombatText::new(format!("{} dies", **actor)),
        valence_color(FctValence::Lethal),
    )
}

fn suppression_line(actor: &LogName) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} is suppressed", **actor)),
        valence_color(FctValence::Suppressed),
    )
}

fn armor_broken_line(actor: &LogName) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{}: armor broken", **actor)),
        valence_color(FctValence::Damage),
    )
}

fn dot_afflicted_line(actor: &LogName, per_turn: DotDamage) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} is afflicted (-{}/turn)", **actor, *per_turn)),
        valence_color(FctValence::Dot),
    )
}

fn field_afflicted_line(actor: &LogName) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} is caught in a hazard field", **actor)),
        valence_color(FctValence::Field),
    )
}

fn bleed_started_line(actor: &LogName) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} is bleeding", **actor)),
        valence_color(FctValence::Status),
    )
}

const fn mode_label(mode: ModeKind) -> &'static str {
    match mode {
        ModeKind::Single => "Single",
        ModeKind::Burst => "Burst",
        ModeKind::Full => "Full",
    }
}

fn fire_declaration_line(actor: &LogName, target: Option<&LogName>, mode: ModeKind) -> LogLine {
    let mode = mode_label(mode);
    let text = match target {
        Some(target) => format!("{} fired {mode} at {}", **actor, **target),
        None => format!("{} fired {mode}", **actor),
    };
    LogLine::new(CombatText::new(text), valence_color(FctValence::Neutral))
}

fn movement_line(actor: &LogName, from: Cell, to: Cell) -> LogLine {
    let text = format!(
        "{} moved ({}, {}) -> ({}, {})",
        **actor, from.x, from.y, to.x, to.y
    );
    LogLine::new(CombatText::new(text), valence_color(FctValence::Neutral))
}

fn shot_outcome_lines(actor: &LogName, report: Option<&HitReport>) -> Vec<LogLine> {
    let Some(report) = report else {
        return Vec::new();
    };
    let pops = classify_report(Some(report));
    if pops.is_empty() {
        return vec![LogLine::new(
            CombatText::new(format!("{} missed", **actor)),
            valence_color(FctValence::Neutral),
        )];
    }
    pops.into_iter()
        .map(|pop| {
            if pop.emphasis() == FctEmphasis::Bold {
                LogLine::new_bold(pop.text().clone(), pop.color())
            } else {
                LogLine::new(pop.text().clone(), pop.color())
            }
        })
        .collect()
}

fn reload_lines(actor: &LogName, outcome: ReloadOutcome) -> Vec<LogLine> {
    match outcome {
        ReloadOutcome::Reloaded => vec![LogLine::new(
            CombatText::new(format!("{} reloaded", **actor)),
            valence_color(FctValence::Neutral),
        )],
        ReloadOutcome::NoTu => vec![LogLine::new(
            CombatText::new(format!("{}: no TU", **actor)),
            valence_color(FctValence::Status),
        )],
        ReloadOutcome::AlreadyFull => Vec::new(),
    }
}

fn turn_line(now_active: Faction, player: PlayerFaction) -> LogLine {
    let label = if now_active == *player {
        "Player"
    } else {
        "Enemy"
    };
    LogLine::new(
        CombatText::new(format!("— {label} turn —")),
        valence_color(FctValence::Neutral),
    )
}
