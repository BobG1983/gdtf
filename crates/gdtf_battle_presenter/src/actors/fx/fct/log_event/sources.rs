//! The per-source [`CombatLogSource`] impls (GTW-572 C5; presenter-side since GTW-620) —
//! one impl per fact message the combat log renders lines from. Each resolves its
//! [`Entity`](bevy::prelude::Entity)s to [`LogName`](super::event::LogName)s AT THIS
//! BOUNDARY (via [`name_of`]) so the shared classifier stays `World`-free.
//!
//! The GTW-572 C6 sources (the Q2 ruling — ALL state changes log): the fall / melee-damage
//! / on-death / suppression / armor-broken gain lines ride
//! [`FallOccurred`] / [`MeleeStruck`] / [`OnDeathOccurred`] / [`SuppressionApplied`] /
//! [`ArmorBroken`]; the DOT / field / bleed afflictions ride their once-per-span START
//! facts ([`DotAfflicted`] / [`FieldAfflicted`] / [`BleedStarted`]) — their per-tick drain
//! signals ([`DotTicked`](gdtf_battle_sim::effects::dot::DotTicked) /
//! [`FieldTicked`](gdtf_battle_sim::effects::fields::FieldTicked) /
//! [`Bleeding`](gdtf_battle_sim::effects::bleed::Bleeding)) have NO impl here, so a mid-affliction tick can
//! never log.

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
    /// `"<actor> fired <Mode> [at <target>]"` — the shot announcement.
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::FireDeclaration {
            actor:  name_of(self.shooter, names),
            target: self.target.map(|t| name_of(t, names)),
            mode:   self.mode,
        })
    }
}

impl CombatLogSource for MovementOccurred {
    /// `"<actor> moved <from> -> <to>"` — the per-step movement line.
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::MovementOccurred {
            actor: name_of(self.actor, names),
            from:  self.from,
            to:    self.to,
        })
    }
}

impl CombatLogSource for MoveRejected {
    /// GTW-537 — only a Suppressed rejection classifies to a line (`"<actor> is pinned"`);
    /// the classifier keeps the other reasons silent, so every rejection forwards.
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::MoveRejected {
            actor:  name_of(self.actor, names),
            reason: self.reason,
        })
    }
}

impl CombatLogSource for ShotImpactResolved {
    /// GTW-328 — each shot's outcome line is built at its OWN (staggered) impact, when the
    /// presenter emits this signal as the bolt lands — so a burst's lines appear
    /// one-per-impact, in cadence. A `None` report (the blast detonation seed) forwards and
    /// classifies to NO line (GTW-559 — a blast is not a miss).
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::ShotOutcome {
            actor:  name_of(self.shooter, names),
            // HitReport is non-`Copy` (it carries the rolled injury) — clone it off the
            // borrowed message and BOX it into the owned event (the variant boxes the
            // report to stay small, clippy `large_enum_variant`).
            report: self.report.clone().map(Box::new),
        })
    }
}

impl CombatLogSource for ReloadResult {
    /// `"<actor> reloaded"` / `"<actor>: no TU"` (an already-full reload classifies to none).
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::ReloadResult {
            actor:   name_of(self.actor, names),
            outcome: self.outcome,
        })
    }
}

impl CombatLogSource for InjuryInflicted {
    /// GTW-439 — `"<actor> <log_text>"` in the severity-scaled wound amber.
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::InjuryInflicted {
            actor:    name_of(self.target, names),
            log_text: InjuryLogText::new((*self.log_text).clone()),
            severity: self.severity,
        })
    }
}

impl CombatLogSource for FallOccurred {
    /// GTW-572 C6 — `"<actor> fell <N> storey(s)"`.
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::FallOccurred {
            actor:   name_of(self.ganger, names),
            storeys: self.storeys,
        })
    }
}

impl CombatLogSource for MeleeStruck {
    /// GTW-572 C6 — `"<attacker> struck <target> (-N)"`: the number-bearing melee fact
    /// GTW-572 added sim-side (the strike-glyph `MeleeResolved` has no actor / amount).
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::MeleeStruck {
            attacker: name_of(self.attacker, names),
            target:   name_of(self.target, names),
            amount:   self.hp_damage,
        })
    }
}

impl CombatLogSource for OnDeathOccurred {
    /// GTW-572 C6 — `"<actor> dies"` for every TERMINAL ganger death (the every-gate signal,
    /// so DOT / field / bleed-out / melee / explode-cascade kills all log). A COVER death
    /// carries [`Entity::PLACEHOLDER`](bevy::prelude::Entity::PLACEHOLDER) — no ganger died,
    /// so it forwards NOTHING (the shot path already logs `"Cover Destroyed"`).
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
    /// GTW-572 C6 — `"<actor> is suppressed"` (the signal GTW-572 extended to carry the
    /// pinned ganger; emitted once per fresh pin, so a refresh never re-logs).
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::SuppressionApplied {
            actor: name_of(self.ganger, names),
        })
    }
}

impl CombatLogSource for ArmorBroken {
    /// GTW-572 C6 — `"<actor>: armor broken"` (the destroy crossing; per-hit wear is silent).
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::ArmorBroken {
            actor: name_of(self.ganger, names),
        })
    }
}

impl CombatLogSource for DotAfflicted {
    /// GTW-572 C6 (the Q2 ruling) — `"<actor> is afflicted (-N/turn)"` ONCE at affliction
    /// start (the fresh-ATTACH fact; the per-tick `DotTicked` has no impl here).
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::DotAfflicted {
            actor:    name_of(self.ganger, names),
            per_turn: self.per_turn,
        })
    }
}

impl CombatLogSource for FieldAfflicted {
    /// GTW-572 C6 (the Q2 ruling) — `"<actor> is caught in a hazard field"` ONCE at
    /// exposure start (the per-round `FieldTicked` has no impl here).
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::FieldAfflicted {
            actor: name_of(self.occupant, names),
        })
    }
}

impl CombatLogSource for BleedStarted {
    /// GTW-572 C6 (the Q2 ruling) — `"<actor> is bleeding"` ONCE at span start (the
    /// per-tick `Bleeding` has no impl here).
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent> {
        Some(CombatLogEvent::BleedStarted {
            actor: name_of(self.ganger, names),
        })
    }
}
