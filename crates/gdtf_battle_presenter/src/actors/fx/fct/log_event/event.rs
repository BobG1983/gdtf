//! The resolved combat-log EVENT vocabulary — [`CombatLogEvent`] (a buffered
//! [`Message`], GTW-572 C5) and the resolved-display newtypes it carries
//! ([`LogName`] / [`InjuryLogText`]).

use bevy::prelude::{Deref, Message};
use gdtf_battle_sim::{
    acts::{MoveRejection, ReloadOutcome},
    battle::PlayerFaction,
    falls::StoreysFallen,
    ganger::GangerName,
    prelude::{Cell, Faction},
    resolve_and_apply::HitReport,
    resolve_hit::HpDamage,
    severity::Severity,
    weapon::{DotDamage, ModeKind},
};

/// A resolved display NAME for a combat-log line — a ganger's name, already looked up from
/// its [`Entity`](bevy::prelude::Entity).
///
/// A NAMED newtype over the displayed [`String`] (no-bare-types: a name shown in the log is
/// a domain value, not a bare `String`), [`Deref`]ing to `str` so the phrasing reads it
/// straight through. The per-source forwarders (co-located in this module since GTW-620)
/// build it from the sim's [`GangerName`] (or a fallback for an unnamed / unresolvable
/// entity); the classifier only ever READS it, so the classifier stays `World`-free.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct LogName(String);

impl LogName {
    /// Build a log name from anything string-like — the forwarder hands in the resolved
    /// [`GangerName`] string (or a fallback for an unnamed actor).
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Build a log name by cloning a sim [`GangerName`] — the common forwarder path
    /// (resolve `Entity` → `&GangerName` → `LogName`).
    #[must_use]
    pub fn from_ganger(name: &GangerName) -> Self {
        // GangerName derefs to its inner String; `**name` is that String (deref the &, then
        // the newtype's Deref), cloned to own it.
        Self((**name).clone())
    }
}

/// A resolved INJURY combat-log clause — the authored
/// [`log_text`](gdtf_battle_sim::acts::InjuryInflicted::log_text), cloned off the message into
/// the log layer's own vocabulary (GTW-439).
///
/// A NAMED newtype over the displayed [`String`], [`Deref`]ing to `str`. The forwarder
/// builds it from the sim's [`LogText`](gdtf_battle_sim::injuries::LogText); the classifier only
/// READS it (a presenter-owned phrasing input, distinct from the sim newtype so the
/// presenter never re-imports the sim type into its enum payload).
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct InjuryLogText(String);

impl InjuryLogText {
    /// Build an injury log clause from anything string-like — the forwarder hands in the
    /// resolved [`LogText`](gdtf_battle_sim::injuries::LogText) string off the
    /// [`InjuryInflicted`](gdtf_battle_sim::acts::InjuryInflicted) message.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

/// A resolved combat event ready to CLASSIFY into log lines — each variant carrying its
/// display data with every [`Entity`](bevy::prelude::Entity) ALREADY resolved to a
/// [`LogName`] / typed value.
///
/// A buffered [`Message`] (GTW-572 C5, `bevy-traps.md` #4): the per-source forwarders
/// (co-located in this module since GTW-620) WRITE it (one thin system per sim signal,
/// resolving names at that boundary) and `gdtf_app`'s ONE appender DRAINS it through
/// [`classify_log_event`](super::classify_log_event) — so adding a log source is a
/// presenter-only change (one forwarder impl + one variant here + one classify arm), never
/// a reader-field / drain-loop / `.clear()` lock-step. Exhaustive: exactly the events the
/// combat log
/// shows. The GTW-572 C6 variants (fall / melee / on-death / suppression / armor-broken /
/// the three affliction starts) deliver the Q2 ruling: EVERY state change logs, and the
/// DOT / field / bleed afflictions log once at their START variant (their per-tick signals
/// have no variant here at all).
///
/// Derives [`PartialEq`] but NOT [`Eq`] (GTW-444): a variant carries a [`HitReport`] whose
/// rolled-injury effects may include a `MovementCostMul` `f32` payload (not `Eq`). Log
/// events are compared with `==` / matched, never keyed in a set.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum CombatLogEvent {
    /// A shot was DECLARED — `"<actor> fired <Mode> at <target>"` (or `"<actor> fired
    /// <Mode>"` when the shot was aimed at no named occupant). From a
    /// [`FireDeclaration`](gdtf_battle_sim::acts::FireDeclaration).
    FireDeclaration {
        /// The shooter's resolved name.
        actor:  LogName,
        /// The struck occupant's resolved name, or [`None`] for a shot at an empty cell /
        /// impact point (the declaration carried no occupant).
        target: Option<LogName>,
        /// The declared fire mode's closed kind — rendered title-cased (`Single` / `Burst`
        /// / `Full`).
        mode:   ModeKind,
    },
    /// A ganger MOVED — `"<actor> moved <from> -> <to>"` (ground cell coords). From a
    /// [`MovementOccurred`](gdtf_battle_sim::acts::MovementOccurred).
    MovementOccurred {
        /// The mover's resolved name.
        actor: LogName,
        /// The ground cell stepped FROM.
        from:  Cell,
        /// The ground cell stepped TO.
        to:    Cell,
    },
    /// A shot's OUTCOME — the `classify_report`
    /// lines (`-7` / `Torso Major` / `DOWN` …), or `"<actor> missed"` for a clean miss
    /// (NEVER suppressed — the user wants misses logged). From the presenter's per-impact
    /// [`ShotImpactResolved`](crate::ShotImpactResolved).
    ShotOutcome {
        /// The shooter's resolved name — used for the `"<actor> missed"` miss line (a
        /// connecting hit's lines come straight from the shared classifier, no name).
        actor:  LogName,
        /// The round's already-computed hit report (the sim's verdict); [`None`] (no
        /// ganger-shot verdict — a blast detonation seed) yields NO line (GTW-559, a blast
        /// is not a miss). BOXED (GTW-438): the [`HitReport`] grew to carry the rolled
        /// injury, so boxing keeps the enum small (clippy `large_enum_variant`).
        report: Option<Box<HitReport>>,
    },
    /// A reload RESOLVED — `"<actor> reloaded"` / `"<actor>: no TU"`; an already-full
    /// reload logs NOTHING. From a [`ReloadResult`](gdtf_battle_sim::acts::ReloadResult).
    ReloadResult {
        /// The reloading ganger's resolved name.
        actor:   LogName,
        /// Which reload branch befell the actor.
        outcome: ReloadOutcome,
    },
    /// A turn BOUNDARY — `"— Player turn —"` / `"— Enemy turn —"`, neutral. From a
    /// [`TurnStarted`](gdtf_battle_sim::turn::TurnStarted).
    TurnStarted {
        /// The faction whose turn just started.
        now_active: Faction,
        /// The player's faction — so the classifier renders the boundary as `Player` (the
        /// player's own turn) vs `Enemy` (any other).
        player:     PlayerFaction,
    },
    /// A MOVE was REJECTED (GTW-537) — only a [`Suppressed`](MoveRejection::Suppressed)
    /// rejection yields a line (`"<actor> is pinned"`, wound AMBER); the
    /// [`Unreachable`](MoveRejection::Unreachable) /
    /// [`Unaffordable`](MoveRejection::Unaffordable) reasons stay silent. From a
    /// [`MoveRejected`](gdtf_battle_sim::acts::MoveRejected).
    MoveRejected {
        /// The rejected mover's resolved name (the log clause's subject).
        actor:  LogName,
        /// Why the commit was rejected — only [`Suppressed`](MoveRejection::Suppressed) logs.
        reason: MoveRejection,
    },
    /// An INJURY was inflicted (GTW-439) — `"<actor> <log_text>"` (e.g. `"Vex loses an
    /// eye"`), drawn in the severity-scaled wound AMBER ramp. From an
    /// [`InjuryInflicted`](gdtf_battle_sim::acts::InjuryInflicted).
    InjuryInflicted {
        /// The wounded ganger's resolved name (the log clause's subject).
        actor:    LogName,
        /// The injury's authored combat-log clause (the predicate — e.g. `"loses an eye"`).
        log_text: InjuryLogText,
        /// The rolled severity bucket — scales the line's wound-AMBER swatch.
        severity: Severity,
    },
    /// A ganger FELL a storey or more (GTW-572 C6) — `"<actor> fell <N> storey(s)"`, wound
    /// AMBER (a harm event). From a [`FallOccurred`](gdtf_battle_sim::falls::FallOccurred).
    FallOccurred {
        /// The fallen ganger's resolved name.
        actor:   LogName,
        /// How many storeys it fell (always ≥ 1).
        storeys: StoreysFallen,
    },
    /// A CONNECTING melee strike applied damage (GTW-572 C6) — `"<attacker> struck
    /// <target> (-N)"`, damage RED. From a [`MeleeStruck`](gdtf_battle_sim::acts::MeleeStruck)
    /// (the number-bearing sim fact GTW-572 added; the strike-glyph `MeleeResolved` carries
    /// no actor / amount).
    MeleeStruck {
        /// The striking ganger's resolved name.
        attacker: LogName,
        /// The struck ganger's resolved name.
        target:   LogName,
        /// The applied HP loss the strike dealt.
        amount:   HpDamage,
    },
    /// A TERMINAL death occurred (GTW-572 C6) — `"<actor> dies"`, lethal RED BOLD. From a
    /// ganger [`OnDeathOccurred`](gdtf_battle_sim::effects::on_death::OnDeathOccurred) (the every-terminal-gate
    /// signal, so the formerly log-invisible DOT / field / bleed-out / melee / explode
    /// cascade kills all log); a COVER death (placeholder entity) is skipped by the
    /// forwarder and never builds this variant.
    OnDeathOccurred {
        /// The dead ganger's resolved name.
        actor: LogName,
    },
    /// A ganger was freshly SUPPRESSED (GTW-572 C6) — `"<actor> is suppressed"`, the cowed
    /// suppression blue-grey. From a
    /// [`SuppressionApplied`](gdtf_battle_sim::suppression::SuppressionApplied) (which GTW-572 extended
    /// to carry the pinned ganger).
    SuppressionApplied {
        /// The pinned ganger's resolved name.
        actor: LogName,
    },
    /// A worn piece BROKE (GTW-572 C6) — `"<actor>: armor broken"`, damage RED (the destroy
    /// crossing reads heavier than wear, matching the FCT tag). From an
    /// [`ArmorBroken`](gdtf_battle_sim::armor_wear::ArmorBroken).
    ArmorBroken {
        /// The ganger whose armor broke, resolved to its name.
        actor: LogName,
    },
    /// A DOT affliction span STARTED (GTW-572 C6, the Q2 ruling) — `"<actor> is afflicted
    /// (-N/turn)"`, the toxic DOT green. From a
    /// [`DotAfflicted`](gdtf_battle_sim::effects::dot::DotAfflicted) (the fresh-ATTACH fact; the per-tick
    /// [`DotTicked`](gdtf_battle_sim::effects::dot::DotTicked) never logs).
    DotAfflicted {
        /// The afflicted ganger's resolved name.
        actor:    LogName,
        /// The affliction's flat per-turn HP drain.
        per_turn: DotDamage,
    },
    /// A field exposure span STARTED (GTW-572 C6, the Q2 ruling) — `"<actor> is caught in a
    /// hazard field"`, the hazard field orange. From a
    /// [`FieldAfflicted`](gdtf_battle_sim::effects::fields::FieldAfflicted) (the once-per-span fact; the
    /// per-round [`FieldTicked`](gdtf_battle_sim::effects::fields::FieldTicked) never logs).
    FieldAfflicted {
        /// The exposed ganger's resolved name.
        actor: LogName,
    },
    /// A bleed affliction span STARTED (GTW-572 C6, the Q2 ruling) — `"<actor> is
    /// bleeding"`, wound AMBER (the FCT tag's family). From a
    /// [`BleedStarted`](gdtf_battle_sim::effects::bleed::BleedStarted) (the once-per-span fact; the
    /// per-tick [`Bleeding`](gdtf_battle_sim::effects::bleed::Bleeding) never logs).
    BleedStarted {
        /// The bleeding ganger's resolved name.
        actor: LogName,
    },
}
