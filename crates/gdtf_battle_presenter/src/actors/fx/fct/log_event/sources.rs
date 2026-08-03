use bevy::prelude::Query;
use gdtf_battle_sim::{
    acts::{
        FireDeclaration, InjuryInflicted, MeleeStruck, MoveRejected, MovementOccurred, ReloadResult,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::BleedStarted, dot::DotAfflicted, fields::FieldAfflicted, on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    ganger::GangerName,
    suppression::SuppressionApplied,
};

use super::{
    event::{CombatLogEvent, InjuryLogText},
    forward::{CombatLogSource, name_of},
};
use crate::ShotImpactResolved;

impl CombatLogSource for FireDeclaration {
        fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::FireDeclaration {
            actor:  name_of(self.shooter, names),
            target: self.target.map(|t| name_of(t, names)),
            mode:   self.mode,
        })
    }
}

impl CombatLogSource for MovementOccurred {
        fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::MovementOccurred {
            actor: name_of(self.actor, names),
            from:  self.from,
            to:    self.to,
        })
    }
}

impl CombatLogSource for MoveRejected {
            fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::MoveRejected {
            actor:  name_of(self.actor, names),
            reason: self.reason,
        })
    }
}

impl CombatLogSource for ShotImpactResolved {
                    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::ShotOutcome {
            actor:  name_of(self.shooter, names),
            report: self.report.clone().map(Box::new),
        })
    }
}

impl CombatLogSource for ReloadResult {
        fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::ReloadResult {
            actor:   name_of(self.actor, names),
            outcome: self.outcome,
        })
    }
}

impl CombatLogSource for InjuryInflicted {
        fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::InjuryInflicted {
            actor:    name_of(self.target, names),
            log_text: InjuryLogText::new((*self.log_text).clone()),
            severity: self.severity,
        })
    }
}

impl CombatLogSource for FallOccurred {
        fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::FallOccurred {
            actor:   name_of(self.ganger, names),
            storeys: self.storeys,
        })
    }
}

impl CombatLogSource for MeleeStruck {
            fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::MeleeStruck {
            attacker: name_of(self.attacker, names),
            target:   name_of(self.target, names),
            amount:   self.hp_damage,
        })
    }
}

impl CombatLogSource for OnDeathOccurred {
                    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        if self.entity == bevy::prelude::Entity::PLACEHOLDER {
            return None;
        }
        Some(CombatLogEvent::OnDeathOccurred {
            actor: name_of(self.entity, names),
        })
    }
}

impl CombatLogSource for SuppressionApplied {
            fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::SuppressionApplied {
            actor: name_of(self.ganger, names),
        })
    }
}

impl CombatLogSource for ArmorBroken {
        fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::ArmorBroken {
            actor: name_of(self.ganger, names),
        })
    }
}

impl CombatLogSource for DotAfflicted {
            fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::DotAfflicted {
            actor:    name_of(self.ganger, names),
            per_turn: self.per_turn,
        })
    }
}

impl CombatLogSource for FieldAfflicted {
            fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::FieldAfflicted {
            actor: name_of(self.occupant, names),
        })
    }
}

impl CombatLogSource for BleedStarted {
            fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::BleedStarted {
            actor: name_of(self.ganger, names),
        })
    }
}
