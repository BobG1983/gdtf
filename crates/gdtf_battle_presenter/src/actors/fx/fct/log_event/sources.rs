use gdtf_battle_sim::{
    acts::{
        FireDeclaration, InjuryInflicted, MeleeStruck, MoveCompleted, MoveRejected, ReloadResult,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::BleedStarted, dot::DotAfflicted, fields::FieldAfflicted, on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    resolve_coarse::ShotKind,
    suppression::SuppressionApplied,
    visibility::{ActObserved, ActVisibility, classify_act},
};

use super::{
    event::{CombatLogEvent, InjuryLogText},
    forward::CombatLogSource,
    sight::PanelSight,
};
use crate::ShotImpactResolved;

impl CombatLogSource for FireDeclaration {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        let landed = self.target.is_some_and(|target| *sight.shows_where(target));
        let observed = ActObserved::new(*sight.shows_where(self.shooter) || landed);
        if classify_act(observed, sight.identifies(self.shooter)) == ActVisibility::Withheld {
            return None;
        }
        Some(CombatLogEvent::FireDeclaration {
            actor:  sight.name_of(self.shooter),
            target: self.target.map(|target| sight.name_of(target)),
            mode:   self.mode,
        })
    }
}

impl CombatLogSource for MoveCompleted {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        if classify_act(sight.shows(self.at), sight.identifies(self.mover))
            == ActVisibility::Withheld
        {
            return None;
        }
        Some(CombatLogEvent::MovementOccurred {
            actor: sight.name_of(self.mover),
        })
    }
}

impl CombatLogSource for MoveRejected {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::MoveRejected {
            actor:  sight.named(self.actor)?,
            reason: self.reason,
        })
    }
}

impl CombatLogSource for ShotImpactResolved {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        let landed = self
            .report
            .as_ref()
            .is_some_and(|report| *impact_shown(sight, report.kind));
        let observed = ActObserved::new(*sight.shows_where(self.shooter) || landed);
        if classify_act(observed, sight.identifies(self.shooter)) == ActVisibility::Withheld {
            return None;
        }
        Some(CombatLogEvent::ShotOutcome {
            actor:  sight.name_of(self.shooter),
            report: self.report.clone().map(Box::new),
        })
    }
}

// Whether the screen is showing wherever the round stopped.
fn impact_shown(sight: &PanelSight, kind: ShotKind) -> ActObserved {
    match kind {
        ShotKind::Ganger(hit) => sight.shows_where(hit),
        ShotKind::Slab(at) | ShotKind::Ground(at) => sight.shows(at),
        ShotKind::Cover(_) | ShotKind::Miss => ActObserved::new(false),
    }
}

impl CombatLogSource for ReloadResult {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::ReloadResult {
            actor:   sight.named(self.actor)?,
            outcome: self.outcome,
        })
    }
}

impl CombatLogSource for InjuryInflicted {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::InjuryInflicted {
            actor:    sight.named(self.target)?,
            log_text: InjuryLogText::new((*self.log_text).clone()),
            severity: self.severity,
        })
    }
}

impl CombatLogSource for FallOccurred {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::FallOccurred {
            actor:   sight.named(self.ganger)?,
            storeys: self.storeys,
        })
    }
}

impl CombatLogSource for MeleeStruck {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        let attacker = sight.about(self.attacker);
        let target = sight.about(self.target);
        if attacker == ActVisibility::Withheld && target == ActVisibility::Withheld {
            return None;
        }
        Some(CombatLogEvent::MeleeStruck {
            attacker: sight.name_of(self.attacker),
            target:   sight.name_of(self.target),
            amount:   self.hp_damage,
        })
    }
}

impl CombatLogSource for OnDeathOccurred {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        if self.entity == bevy::prelude::Entity::PLACEHOLDER {
            return None;
        }
        Some(CombatLogEvent::OnDeathOccurred {
            actor: sight.named(self.entity)?,
        })
    }
}

impl CombatLogSource for SuppressionApplied {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::SuppressionApplied {
            actor: sight.named(self.ganger)?,
        })
    }
}

impl CombatLogSource for ArmorBroken {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::ArmorBroken {
            actor: sight.named(self.ganger)?,
        })
    }
}

impl CombatLogSource for DotAfflicted {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::DotAfflicted {
            actor:    sight.named(self.ganger)?,
            per_turn: self.per_turn,
        })
    }
}

impl CombatLogSource for FieldAfflicted {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::FieldAfflicted {
            actor: sight.named(self.occupant)?,
        })
    }
}

impl CombatLogSource for BleedStarted {
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::BleedStarted {
            actor: sight.named(self.ganger)?,
        })
    }
}
