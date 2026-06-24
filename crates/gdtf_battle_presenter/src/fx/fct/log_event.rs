//! GTW-328 (slice 2): the SHARED combat-event → text CLASSIFICATION layer for the
//! battlescape combat-text LOG.
//!
//! The combat log is the bottom-left HUD strip of recent combat events that scroll up and
//! fade (movement, a shot declaration, the shot's outcome, a reload, a turn boundary). The
//! sim already emits the five backing signals (slice 1 — [`FireDeclaration`], [`MovementOccurred`],
//! [`ShotFired`], [`ReloadResult`], [`TurnStarted`]); this module is the PURE function that
//! turns ONE such event — with its [`Entity`](bevy::prelude::Entity) handles ALREADY resolved
//! to display data — into the ordered [`LogLine`]s the log renders.
//!
//! The reader system (slice 3) does the World-touching half: it drains a sim message, resolves
//! each [`Entity`](bevy::prelude::Entity) to a name via `Query<&`[`GangerName`](gdtf_battle_sim::GangerName)`>`,
//! builds the matching [`CombatLogEvent`], and hands it here. Because every variant carries the
//! RESOLVED display data (name strings as [`LogName`], typed [`Cell`] / [`ModeKind`] /
//! [`Faction`] / [`ReloadOutcome`] values — never a raw
//! [`Entity`](bevy::prelude::Entity)), [`classify_log_event`] is a pure function unit-testable
//! with NO [`App`](bevy::prelude::App) / [`World`](bevy::prelude::World).
//!
//! The shot OUTCOME reuses the FCT classifier verbatim: [`classify_report`](super::reader::classify_report)
//! already maps a [`HitReport`](gdtf_battle_sim::HitReport) to the damage / wound / DOWN / DEAD
//! flesh pops AND the cover / slab / ground STRUCTURAL pops (GTW-302 / GTW-327 / GTW-386), so
//! [`CombatLogEvent::ShotOutcome`] feeds the same report through it and renders those classified
//! pops as log lines — the outcome → text mapping lives in ONE place, never duplicated (its FCT
//! callers are untouched). So a structural hit logs a real line (`"Cover hit"` / `"Slab
//! Destroyed"` / `"Dust"` …), NOT a phantom miss. Only a GENUINE clean miss (a report that
//! yields no pops) reads `"<name> missed"`: the user explicitly wants misses in the log, so a
//! miss is NEVER suppressed here (unlike the floating-combat-text, which drops a clean miss).
//!
//! Pure VIEW (ADR-0001): these are presenter-owned phrasing + palette decisions over resolved
//! data; they read no sim state and write nothing.

use bevy::prelude::*;
use gdtf_battle_sim::{
    Cell, Faction, GangerName, HitReport, ModeKind, PlayerFaction, ReloadOutcome,
};

use super::{
    palette::{FctValence, valence_color},
    reader::classify_report,
    text::{CombatText, FctEmphasis},
};

/// A resolved display NAME for a combat-log line — a ganger's name, already looked up from its
/// [`Entity`](bevy::prelude::Entity).
///
/// A NAMED newtype over the displayed [`String`] (no-bare-types: a name shown in the log is a
/// domain value, not a bare `String`), [`Deref`]ing to `str` so the phrasing reads it straight
/// through. The slice-3 reader builds it from the sim's
/// [`GangerName`](gdtf_battle_sim::GangerName) (or a fallback for an unnamed / unresolvable
/// entity); the classifier only ever READS it, so the classifier stays World-free.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct LogName(String);

impl LogName {
    /// Build a log name from anything string-like — the reader hands in the resolved
    /// [`GangerName`](gdtf_battle_sim::GangerName) string (or a fallback for an unnamed actor).
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Build a log name by cloning a sim [`GangerName`](gdtf_battle_sim::GangerName) — the
    /// common reader path (resolve `Entity` → `&GangerName` → `LogName`).
    #[must_use]
    pub fn from_ganger(name: &GangerName) -> Self {
        // GangerName derefs to its inner String; `**name` is that String (deref the &, then
        // the newtype's Deref), cloned to own it.
        Self((**name).clone())
    }
}

/// A resolved combat event ready to CLASSIFY into log lines — the five Phase-1 combat-log
/// events, each carrying its display data with every [`Entity`](bevy::prelude::Entity) ALREADY
/// resolved to a [`LogName`] / typed value.
///
/// A named domain enum (no-bare-types: a combat-log event is a domain value). Mirrors the five
/// slice-1 sim signals — [`FireDeclaration`](gdtf_battle_sim::FireDeclaration),
/// [`MovementOccurred`](gdtf_battle_sim::MovementOccurred),
/// [`ShotFired`](gdtf_battle_sim::ShotFired), [`ReloadResult`](gdtf_battle_sim::ReloadResult),
/// [`TurnStarted`](gdtf_battle_sim::TurnStarted) — but holds RESOLVED display data, NOT raw
/// entities, so [`classify_log_event`] is pure and World-free (the slice-3 reader does the
/// `Entity` → name resolution). Exhaustive: exactly the events the combat log shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombatLogEvent {
    /// A shot was DECLARED — `"<actor> fired <Mode> at <target>"` (or `"<actor> fired <Mode>"`
    /// when the shot was aimed at no named occupant). From a
    /// [`FireDeclaration`](gdtf_battle_sim::FireDeclaration).
    FireDeclaration {
        /// The shooter's resolved name.
        actor:  LogName,
        /// The struck occupant's resolved name, or [`None`] for a shot at an empty cell /
        /// impact point (the declaration carried no occupant).
        target: Option<LogName>,
        /// The declared fire mode's closed kind — rendered title-cased (`Single` / `Burst` /
        /// `Full`).
        mode:   ModeKind,
    },
    /// A ganger MOVED — `"<actor> moved <from> -> <to>"` (ground cell coords). From a
    /// [`MovementOccurred`](gdtf_battle_sim::MovementOccurred).
    MovementOccurred {
        /// The mover's resolved name.
        actor: LogName,
        /// The ground cell stepped FROM.
        from:  Cell,
        /// The ground cell stepped TO.
        to:    Cell,
    },
    /// A shot's OUTCOME — the [`classify_report`] lines (`-7` / `Torso Major` / `DOWN` …), or
    /// `"<actor> missed"` for a clean miss (NEVER suppressed — the user wants misses logged).
    /// From a [`ShotFired`](gdtf_battle_sim::ShotFired) (its
    /// [`report`](gdtf_battle_sim::ShotFired::report)).
    ShotOutcome {
        /// The shooter's resolved name — used for the `"<actor> missed"` miss line (a
        /// connecting hit's lines come straight from [`classify_report`], no name).
        actor:  LogName,
        /// The round's already-computed hit report (the sim's verdict) — reused through
        /// [`classify_report`]; [`None`] (a geometry-only round) reads as a miss.
        report: Option<HitReport>,
    },
    /// A reload RESOLVED — `"<actor> reloaded"` / `"<actor>: no TU"`; an already-full reload
    /// logs NOTHING. From a [`ReloadResult`](gdtf_battle_sim::ReloadResult).
    ReloadResult {
        /// The reloading ganger's resolved name.
        actor:   LogName,
        /// Which reload branch befell the actor.
        outcome: ReloadOutcome,
    },
    /// A turn BOUNDARY — `"— Player turn —"` / `"— Enemy turn —"`, neutral. From a
    /// [`TurnStarted`](gdtf_battle_sim::TurnStarted).
    TurnStarted {
        /// The faction whose turn just started.
        now_active: Faction,
        /// The player's faction — so the classifier renders the boundary as `Player` (the
        /// player's own turn) vs `Enemy` (any other).
        player:     PlayerFaction,
    },
}

/// One rendered combat-log line — its text, its valence color, and its emphasis weight.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color, FctEmphasis)` tuple): mirrors the
/// FCT [`ClassifiedPop`](super::reader::ClassifiedPop) shape but is the log layer's unit — one
/// scroll-up-and-fade line. Reuses the existing FCT vocabulary: [`CombatText`] for the string,
/// the [`FctValence`] palette via [`valence_color`] for the color, and [`FctEmphasis`] for the
/// weight (the lethal DOWN / DEAD line rides [`FctEmphasis::Bold`]). The
/// [`Color`](bevy::prelude::Color) is framework plumbing (the swatch the renderer draws), the
/// only bare type the no-bare-types rule permits here.
///
/// `pub`: the slice-3 reader builds [`CombatLogEvent`]s and the slice-4 log presenter renders
/// the [`LogLine`]s — read through the [`text`](Self::text) / [`color`](Self::color) /
/// [`emphasis`](Self::emphasis) accessors (the fields stay private so a line is only built
/// through [`classify_log_event`], never field-assembled outside).
#[derive(Debug, Clone, PartialEq)]
pub struct LogLine {
    /// The combat-text string this line renders.
    text:     CombatText,
    /// The valence swatch the line is drawn in.
    color:    Color,
    /// The styling weight the line is drawn at ([`FctEmphasis::Bold`] for the lethal line).
    emphasis: FctEmphasis,
}

impl LogLine {
    /// Build an ordinary (body-weight) log line from its text + valence color.
    #[must_use]
    const fn new(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Normal,
        }
    }

    /// Build an EMPHASIZED (bold) log line from its text + valence color — the lethal DOWN /
    /// DEAD line, the heaviest in the blood family.
    #[must_use]
    const fn new_bold(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Bold,
        }
    }

    /// The line's combat-text string.
    #[must_use]
    pub const fn text(&self) -> &CombatText {
        &self.text
    }

    /// The line's valence swatch — the [`Color`](bevy::prelude::Color) the renderer draws.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// The line's styling weight ([`FctEmphasis::Bold`] for the lethal line, else
    /// [`FctEmphasis::Normal`]).
    #[must_use]
    pub const fn emphasis(&self) -> FctEmphasis {
        self.emphasis
    }
}

/// Classify ONE resolved [`CombatLogEvent`] into the ordered [`LogLine`]s the combat log shows.
///
/// The single PURE phrasing + palette layer: it maps each event to its battle-HUD line(s) and
/// never touches a [`World`](bevy::prelude::World) (the reader resolves the entities first), so
/// every variant is unit-testable in isolation. Most events yield exactly one line; a
/// [`ShotOutcome`](CombatLogEvent::ShotOutcome) on a CONNECTING hit yields the
/// [`classify_report`] list (one line per HP / wound / penetration / DOWN-DEAD event), and an
/// [`AlreadyFull`](ReloadOutcome::AlreadyFull) reload yields NONE (a no-op reload is not worth
/// a log line).
///
/// Phrasing (v1 battle-HUD voice):
///
/// - **Fire declaration** → `"<actor> fired <Mode> at <target>"` (`<Mode>` title-cased), or
///   `"<actor> fired <Mode>"` when no named target. Neutral GREY (it is an announcement, not a
///   result).
/// - **Movement** → `"<actor> moved <from> -> <to>"` (ground cell coords). Neutral GREY.
/// - **Shot outcome** → the [`classify_report`] lines (REUSED — the flesh AND structural
///   outcome mapping is not duplicated), carrying each pop's color + emphasis; a structural
///   cover / slab / ground hit logs its real `"Cover hit"` / `"Slab Destroyed"` / `"Dust"` line
///   (GTW-386), and only a genuine clean MISS (no pops) reads `"<actor> missed"` in neutral GREY
///   (misses are NEVER suppressed).
/// - **Reload** → `"<actor> reloaded"` (neutral GREY) / `"<actor>: no TU"` (a denied act —
///   AMBER); [`AlreadyFull`](ReloadOutcome::AlreadyFull) → no line.
/// - **Turn** → `"— Player turn —"` / `"— Enemy turn —"`, neutral GREY.
///
/// `pub`: the slice-4 log presenter calls it on each freshly-built [`CombatLogEvent`].
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
        CombatLogEvent::ShotOutcome { actor, report } => shot_outcome_lines(actor, report.as_ref()),
        CombatLogEvent::ReloadResult { actor, outcome } => reload_lines(actor, *outcome),
        CombatLogEvent::TurnStarted { now_active, player } => {
            vec![turn_line(*now_active, *player)]
        }
    }
}

/// The title-cased fire-mode label for the log (`"Single"` / `"Burst"` / `"Full"`).
///
/// The combat log uses a terse title-cased mode word, distinct from [`ModeKind`]'s own
/// lower-case [`Display`](std::fmt::Display) (`"single"` / `"burst"` / `"full-auto"`), which is
/// the weapon-picker label — the log voice is short and capitalized.
const fn mode_label(mode: ModeKind) -> &'static str {
    match mode {
        ModeKind::Single => "Single",
        ModeKind::Burst => "Burst",
        ModeKind::Full => "Full",
    }
}

/// The fire-declaration line — `"<actor> fired <Mode> at <target>"`, or `"<actor> fired <Mode>"`
/// when the shot named no target. Neutral GREY (an announcement, not a result).
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
/// (a flesh hit OR a structural cover / slab / ground hit), or the `"<actor> missed"` miss line
/// for a genuine clean miss (NEVER suppressed).
///
/// A connecting hit's lines come verbatim from [`classify_report`] (each pop's text + color +
/// emphasis), so the outcome → text mapping is shared with the floating-combat-text and never
/// duplicated — a structural hit therefore logs its real `"Cover hit"` / `"Slab Destroyed"` /
/// `"Dust"` line (GTW-386), not a phantom miss. ONLY when the report yields NO pops (a genuine
/// clean miss, or a geometry-only `None` report) does the log instead show the explicit miss
/// line the user asked for.
fn shot_outcome_lines(actor: &LogName, report: Option<&HitReport>) -> Vec<LogLine> {
    let pops = classify_report(report);
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
/// act); an [`AlreadyFull`](ReloadOutcome::AlreadyFull) reload yields NO line (a no-op is not
/// worth logging).
fn reload_lines(actor: &LogName, outcome: ReloadOutcome) -> Vec<LogLine> {
    match outcome {
        ReloadOutcome::Reloaded => vec![LogLine::new(
            CombatText::new(format!("{} reloaded", **actor)),
            valence_color(FctValence::Neutral),
        )],
        ReloadOutcome::NoTu => vec![LogLine::new(
            CombatText::new(format!("{}: no TU", **actor)),
            valence_color(FctValence::Wound),
        )],
        // An already-full magazine is a no-op (acts/reload.rs charges nothing) — no log line.
        ReloadOutcome::AlreadyFull => Vec::new(),
    }
}

/// The turn-boundary line — `"— Player turn —"` when the newly-active faction is the player's,
/// else `"— Enemy turn —"`. Neutral GREY (a boundary marker, not a combat result).
fn turn_line(now_active: Faction, player: PlayerFaction) -> LogLine {
    // PlayerFaction derefs to the player's Faction; the boundary reads "Player" only when the
    // turn passed to that gang, else "Enemy" (any other gang).
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

#[cfg(test)]
mod test {
    use bevy::prelude::Entity;
    use gdtf_battle_sim::{
        AppliedDamage, ArmorHardness, ArmorProtection, BodyPart, Cell, CellLevel, CoverEntry,
        CoverHp, Faction, HeightBand, HitReport, HitResult, HpDamage, IntegrityWear, Level,
        LifeState, Matchup, ModeKind, PenetratingDamage, PlayerFaction, ReloadOutcome, Severity,
        ShotKind,
    };

    use super::{
        CombatLogEvent, FctEmphasis, FctValence, LogName, classify_log_event, valence_color,
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
            // classify_report only matches the Ganger variant, never derefs the entity.
            kind:            ShotKind::Ganger(Entity::PLACEHOLDER),
            part:            Some(part),
            applied:         Some(AppliedDamage {
                matchup: Matchup::Neutral,
                hit: HitResult {
                    penetrating: PenetratingDamage::new(pen),
                    hp_damage:   HpDamage::new(hp),
                    wear:        IntegrityWear::new(0),
                },
                severity,
                life_after,
                broken: None,
                worn: None,
            }),
            cover_destroyed: None,
            slab_destroyed:  None,
            ground_accrued:  None,
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
            report: Some(report),
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
            report: Some(HitReport::no_effect(ShotKind::Miss)),
        };
        let lines = classify_log_event(&event);
        assert_eq!(lines.len(), 1, "a clean miss is exactly one log line");
        assert_eq!(&**lines[0].text(), "Vex missed");
        assert_eq!(lines[0].color(), valence_color(FctValence::Neutral));

        // A geometry-only None report is likewise a miss.
        let none_event = CombatLogEvent::ShotOutcome {
            actor:  LogName::new("Vex"),
            report: None,
        };
        let none_lines = classify_log_event(&none_event);
        assert_eq!(none_lines.len(), 1);
        assert_eq!(&**none_lines[0].text(), "Vex missed");
    }

    /// GTW-386 — a COVER hit logs a real structural line, NOT `"<actor> missed"`: a damaging hit
    /// reads the `"Cover hit"` chip line, and a DESTROYING hit reads the lethal-RED BOLD
    /// `"Cover Destroyed"` line. PIN-DISCRIMINATING: with the old classifier (non-ganger → empty
    /// pop list) `shot_outcome_lines` would fall through to `"Vex missed"` and BOTH asserts fail.
    #[test]
    fn a_cover_hit_logs_a_structural_line_not_a_miss() {
        // Damaged (not destroyed) — a "Cover hit" chip line, never the miss line.
        let damaged = CombatLogEvent::ShotOutcome {
            actor:  LogName::new("Vex"),
            report: Some(HitReport::no_effect(ShotKind::Cover(cover_entry()))),
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
            cover_destroyed: Some(struck_key()),
            ..HitReport::no_effect(ShotKind::Cover(cover_entry()))
        };
        let destroyed = CombatLogEvent::ShotOutcome {
            actor:  LogName::new("Vex"),
            report: Some(destroyed_report),
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
            slab_destroyed: Some(struck_key()),
            ..HitReport::no_effect(ShotKind::Slab(struck_key()))
        };
        let event = CombatLogEvent::ShotOutcome {
            actor:  LogName::new("Vex"),
            report: Some(report),
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
        assert_eq!(no_tu_lines[0].color(), valence_color(FctValence::Wound));

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
}
