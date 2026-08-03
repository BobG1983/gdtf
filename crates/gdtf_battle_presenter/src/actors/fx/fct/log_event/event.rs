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

#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct LogName(String);

impl LogName {
            #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

            #[must_use]
    pub fn from_ganger(name: &GangerName) -> Self {
        Self((**name).clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct InjuryLogText(String);

impl InjuryLogText {
                #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

#[derive(Message, Debug, Clone, PartialEq)]
pub enum CombatLogEvent {
                FireDeclaration {
                actor:  LogName,
                        target: Option<LogName>,
                        mode:   ModeKind,
    },
            MovementOccurred {
                actor: LogName,
                from:  Cell,
                to:    Cell,
    },
                    ShotOutcome {
                        actor:  LogName,
                                        report: Option<Box<HitReport>>,
    },
            ReloadResult {
                actor:   LogName,
                outcome: ReloadOutcome,
    },
            TurnStarted {
                now_active: Faction,
                        player:     PlayerFaction,
    },
                        MoveRejected {
                actor:  LogName,
                reason: MoveRejection,
    },
                InjuryInflicted {
                actor:    LogName,
                log_text: InjuryLogText,
                severity: Severity,
    },
            FallOccurred {
                actor:   LogName,
                storeys: StoreysFallen,
    },
                    MeleeStruck {
                attacker: LogName,
                target:   LogName,
                amount:   HpDamage,
    },
                        OnDeathOccurred {
                actor: LogName,
    },
                    SuppressionApplied {
                actor: LogName,
    },
                ArmorBroken {
                actor: LogName,
    },
                    DotAfflicted {
                actor:    LogName,
                per_turn: DotDamage,
    },
                    FieldAfflicted {
                actor: LogName,
    },
                    BleedStarted {
                actor: LogName,
    },
}
