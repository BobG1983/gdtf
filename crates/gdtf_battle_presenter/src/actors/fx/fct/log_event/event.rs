//! Combat log event enum and name helpers.

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

/// Display name for a combat log line.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct LogName(String);

impl LogName {
    /// Build from any string-like value.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Build from a ganger name component.
    #[must_use]
    pub fn from_ganger(name: &GangerName) -> Self {
        Self((**name).clone())
    }
}

/// Injury description text for the log.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct InjuryLogText(String);

impl InjuryLogText {
    /// Build from any string-like value.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

/// View-side combat log events derived from played acts.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum CombatLogEvent {
    /// Someone declared fire.
    FireDeclaration {
        /// Shooter name.
        actor: LogName,
        /// Optional target name.
        target: Option<LogName>,
        /// Fire mode used.
        mode: ModeKind,
    },
    /// Someone walked between cells.
    MovementOccurred {
        /// Walker name.
        actor: LogName,
        /// Start cell.
        from: Cell,
        /// End cell.
        to: Cell,
    },
    /// A round resolved with an optional hit report.
    ShotOutcome {
        /// Shooter name.
        actor: LogName,
        /// Hit report if any.
        report: Option<Box<HitReport>>,
    },
    /// Reload finished.
    ReloadResult {
        /// Reloader name.
        actor: LogName,
        /// Reload outcome.
        outcome: ReloadOutcome,
    },
    /// Turn boundary.
    TurnStarted {
        /// Faction now active.
        now_active: Faction,
        /// Player faction.
        player: PlayerFaction,
    },
    /// Move was refused.
    MoveRejected {
        /// Actor name.
        actor: LogName,
        /// Refusal reason.
        reason: MoveRejection,
    },
    /// Injury inflicted.
    InjuryInflicted {
        /// Victim name.
        actor: LogName,
        /// Injury log text.
        log_text: InjuryLogText,
        /// Injury severity.
        severity: Severity,
    },
    /// Fall occurred.
    FallOccurred {
        /// Fallen ganger name.
        actor: LogName,
        /// Storeys fallen.
        storeys: StoreysFallen,
    },
    /// Melee hit landed.
    MeleeStruck {
        /// Attacker name.
        attacker: LogName,
        /// Target name.
        target: LogName,
        /// HP damage.
        amount: HpDamage,
    },
    /// On-death trigger fired.
    OnDeathOccurred {
        /// Dead ganger name.
        actor: LogName,
    },
    /// Suppression applied.
    SuppressionApplied {
        /// Suppressed ganger name.
        actor: LogName,
    },
    /// Armor plate broke.
    ArmorBroken {
        /// Wearer name.
        actor: LogName,
    },
    /// Dot affliction started.
    DotAfflicted {
        /// Victim name.
        actor: LogName,
        /// Damage per turn.
        per_turn: DotDamage,
    },
    /// Field affliction started.
    FieldAfflicted {
        /// Victim name.
        actor: LogName,
    },
    /// Bleed started.
    BleedStarted {
        /// Victim name.
        actor: LogName,
    },
}
