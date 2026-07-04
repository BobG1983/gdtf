//! The single PURE phrasing + palette layer — [`classify_log_event`] and its per-family
//! line helpers (GTW-328 slice 2; the GTW-572 C6 state-change arms).

use gdtf_battle_sim::{
    Cell, DotDamage, Faction, HitReport, HpDamage, ModeKind, MoveRejection, PlayerFaction,
    ReloadOutcome, Severity, StoreysFallen,
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

/// Classify ONE resolved [`CombatLogEvent`] into the ordered [`LogLine`]s the combat log
/// shows.
///
/// The single PURE phrasing + palette layer (the ONE view-side match per family — GTW-572
/// P3): it maps each event to its battle-HUD line(s) and never touches a
/// [`World`](bevy::prelude::World) (the forwarders resolve the entities first), so every
/// variant is unit-testable in isolation. Most events yield exactly one line; a
/// [`ShotOutcome`](CombatLogEvent::ShotOutcome) on a CONNECTING hit yields the
/// [`classify_report`] list, an [`AlreadyFull`](gdtf_battle_sim::ReloadOutcome::AlreadyFull)
/// reload and a non-suppressed move rejection yield NONE.
///
/// Phrasing (v1 battle-HUD voice):
///
/// - **Fire declaration** → `"<actor> fired <Mode> at <target>"` (neutral GREY).
/// - **Movement** → `"<actor> moved <from> -> <to>"` (neutral GREY).
/// - **Shot outcome** → the [`classify_report`] lines (REUSED — never duplicated); only a
///   genuine clean MISS reads `"<actor> missed"` (neutral GREY, NEVER suppressed); a
///   `None` report — a blast detonation seed — yields NO line (GTW-559).
/// - **Reload** → `"<actor> reloaded"` (GREY) / `"<actor>: no TU"` (AMBER); already-full →
///   no line.
/// - **Turn** → `"— Player turn —"` / `"— Enemy turn —"` (GREY).
/// - **Move rejected** → `"<actor> is pinned"` (AMBER) for Suppressed only.
/// - **Injury** → `"<actor> <log_text>"` in the severity-scaled AMBER ramp.
/// - **Fall** (GTW-572) → `"<actor> fell <N> storey(s)"` (AMBER — a harm event).
/// - **Melee damage** (GTW-572) → `"<attacker> struck <target> (-N)"` (damage RED).
/// - **Death** (GTW-572) → `"<actor> dies"` (lethal RED, BOLD — the line mirror of the
///   DOWN / DEAD tag, but named).
/// - **Suppression** (GTW-572) → `"<actor> is suppressed"` (the cowed blue-grey).
/// - **Armor broken** (GTW-572) → `"<actor>: armor broken"` (damage RED).
/// - **DOT afflicted** (GTW-572) → `"<actor> is afflicted (-N/turn)"` (toxic DOT green).
/// - **Field afflicted** (GTW-572) → `"<actor> is caught in a hazard field"` (hazard orange).
/// - **Bleed started** (GTW-572) → `"<actor> is bleeding"` (wound AMBER).
///
/// `pub`: the app-side appender calls it on each drained [`CombatLogEvent`].
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

/// The move-rejected lines (GTW-537) — a [`Suppressed`](MoveRejection::Suppressed) rejection
/// reads `"<actor> is pinned"` in wound AMBER (a denied act, the `"no TU"` reload precedent);
/// the [`Unreachable`](MoveRejection::Unreachable) / [`Unaffordable`](MoveRejection::Unaffordable)
/// reasons yield NO line (unsurfaced pre-GTW-537, kept silent — the strip is not spammed
/// with every mis-click).
fn move_rejected_lines(actor: &LogName, reason: MoveRejection) -> Vec<LogLine> {
    match reason {
        MoveRejection::Suppressed => vec![LogLine::new(
            CombatText::new(format!("{} is pinned", **actor)),
            valence_color(FctValence::Status),
        )],
        // The pre-existing reasons stay silent (unsurfaced today) — no line.
        MoveRejection::Unreachable | MoveRejection::Unaffordable => Vec::new(),
    }
}

/// The injury line — `"<actor> <log_text>"` (e.g. `"Vex loses an eye"`), drawn in the
/// severity-scaled wound AMBER ramp so a worse injury reads hotter (GTW-439).
fn injury_line(actor: &LogName, log_text: &InjuryLogText, severity: Severity) -> LogLine {
    let text = format!("{} {}", **actor, **log_text);
    LogLine::new(CombatText::new(text), severity_color(severity))
}

/// The fall line (GTW-572 C6) — `"<actor> fell <N> storey(s)"` in wound AMBER: a fall is a
/// harm event (its damage / wounds ride their own signals), phrased with the typed storey
/// count so a bigger drop reads bigger.
fn fall_line(actor: &LogName, storeys: StoreysFallen) -> LogLine {
    let count = *storeys;
    let noun = if count == 1 { "storey" } else { "storeys" };
    LogLine::new(
        CombatText::new(format!("{} fell {count} {noun}", **actor)),
        valence_color(FctValence::Status),
    )
}

/// The melee-damage line (GTW-572 C6) — `"<attacker> struck <target> (-N)"` in the damage
/// RED: the number-bearing melee fact ([`MeleeStruck`](gdtf_battle_sim::MeleeStruck))
/// finally surfaces the strike's applied HP loss in the log.
fn melee_struck_line(attacker: &LogName, target: &LogName, amount: HpDamage) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} struck {} (-{})", **attacker, **target, *amount)),
        valence_color(FctValence::Damage),
    )
}

/// The named death line (GTW-572 C6) — `"<actor> dies"` in the lethal RED, BOLD: the line
/// mirror of the shot classifier's DOWN / DEAD tag, but NAMED — and emitted for EVERY
/// terminal death (DOT / field / bleed-out / melee / explode-cascade kills included, the
/// formerly log-invisible ones).
fn death_line(actor: &LogName) -> LogLine {
    LogLine::new_bold(
        CombatText::new(format!("{} dies", **actor)),
        valence_color(FctValence::Lethal),
    )
}

/// The suppression line (GTW-572 C6) — `"<actor> is suppressed"` in the cowed suppression
/// blue-grey (the FCT `"SUPPRESSED"` tag's family, so pop and line read as one signal).
fn suppression_line(actor: &LogName) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} is suppressed", **actor)),
        valence_color(FctValence::Suppressed),
    )
}

/// The armor-broken line (GTW-572 C6) — `"<actor>: armor broken"` in the damage RED (the
/// destroy crossing reads heavier than wear, matching the FCT `"Armor Broken"` tag).
fn armor_broken_line(actor: &LogName) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{}: armor broken", **actor)),
        valence_color(FctValence::Damage),
    )
}

/// The DOT affliction-start line (GTW-572 C6, the Q2 ruling) — `"<actor> is afflicted
/// (-N/turn)"` in the toxic DOT green, ONCE per affliction span (the per-tick drain never
/// logs; its FCT pop carries the per-round number).
fn dot_afflicted_line(actor: &LogName, per_turn: DotDamage) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} is afflicted (-{}/turn)", **actor, *per_turn)),
        valence_color(FctValence::Dot),
    )
}

/// The field exposure-start line (GTW-572 C6, the Q2 ruling) — `"<actor> is caught in a
/// hazard field"` in the hazard field orange, ONCE per exposure span.
fn field_afflicted_line(actor: &LogName) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} is caught in a hazard field", **actor)),
        valence_color(FctValence::Field),
    )
}

/// The bleed affliction-start line (GTW-572 C6, the Q2 ruling) — `"<actor> is bleeding"` in
/// the wound AMBER (the FCT `"Bleeding"` tag's family), ONCE per bleed span.
fn bleed_started_line(actor: &LogName) -> LogLine {
    LogLine::new(
        CombatText::new(format!("{} is bleeding", **actor)),
        valence_color(FctValence::Status),
    )
}

/// The title-cased fire-mode label for the log (`"Single"` / `"Burst"` / `"Full"`).
///
/// The combat log uses a terse title-cased mode word, distinct from [`ModeKind`]'s own
/// lower-case [`Display`](std::fmt::Display) (the weapon-picker label) — the log voice is
/// short and capitalized.
const fn mode_label(mode: ModeKind) -> &'static str {
    match mode {
        ModeKind::Single => "Single",
        ModeKind::Burst => "Burst",
        ModeKind::Full => "Full",
    }
}

/// The fire-declaration line — `"<actor> fired <Mode> at <target>"`, or `"<actor> fired
/// <Mode>"` when the shot named no target. Neutral GREY (an announcement, not a result).
fn fire_declaration_line(actor: &LogName, target: Option<&LogName>, mode: ModeKind) -> LogLine {
    let mode = mode_label(mode);
    let text = match target {
        Some(target) => format!("{} fired {mode} at {}", **actor, **target),
        None => format!("{} fired {mode}", **actor),
    };
    LogLine::new(CombatText::new(text), valence_color(FctValence::Neutral))
}

/// The movement line — `"<actor> moved <from> -> <to>"` (ground cell coords). Neutral GREY.
fn movement_line(actor: &LogName, from: Cell, to: Cell) -> LogLine {
    let text = format!(
        "{} moved ({}, {}) -> ({}, {})",
        **actor, from.x, from.y, to.x, to.y
    );
    LogLine::new(CombatText::new(text), valence_color(FctValence::Neutral))
}

/// The shot-outcome lines — the [`classify_report`] pops (REUSED) for any connecting hit
/// (a flesh hit OR a structural cover / slab / ground hit), or the `"<actor> missed"` miss
/// line for a genuine clean miss (NEVER suppressed).
///
/// A connecting hit's lines come verbatim from [`classify_report`] (each pop's text +
/// color + emphasis), so the outcome → text mapping is shared with the floating-combat-text
/// and never duplicated — a structural hit logs its real `"Cover hit"` / `"Slab Destroyed"`
/// / `"Dust"` line (GTW-386), not a phantom miss. ONLY when a CARRIED report yields NO pops
/// (a genuine clean miss) does the log show the explicit miss line. A `None` report yields
/// NO line at all (GTW-559): "missed" is a ganger-shot VERDICT, and a verdict-less impact —
/// the grenade blast's detonation seed, the only `None` producer on this path — has no
/// hit-or-miss outcome to log, so rendering it as a miss was the phantom `"Someone missed"`
/// bug.
fn shot_outcome_lines(actor: &LogName, report: Option<&HitReport>) -> Vec<LogLine> {
    // GTW-559: no report ⇒ no ganger-shot verdict ⇒ no outcome line. Only the blast's
    // detonation seed produces a None report on this path (every fired volley round carries
    // Some — a clean miss included), so this never suppresses a real miss.
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

/// The reload lines — `"<actor> reloaded"` (neutral) / `"<actor>: no TU"` (AMBER, a denied
/// act); an [`AlreadyFull`](ReloadOutcome::AlreadyFull) reload yields NO line (a no-op is
/// not worth logging).
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
        // An already-full magazine is a no-op (acts/reload.rs charges nothing) — no log line.
        ReloadOutcome::AlreadyFull => Vec::new(),
    }
}

/// The turn-boundary line — `"— Player turn —"` when the newly-active faction is the
/// player's, else `"— Enemy turn —"`. Neutral GREY (a boundary marker, not a combat result).
fn turn_line(now_active: Faction, player: PlayerFaction) -> LogLine {
    // PlayerFaction derefs to the player's Faction; the boundary reads "Player" only when
    // the turn passed to that gang, else "Enemy" (any other gang).
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
