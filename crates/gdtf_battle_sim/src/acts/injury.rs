//! Apply rolled injuries onto a target's injury ledger.

use bevy::prelude::{Commands, Entity, Message, MessageReader, Query};

use crate::{
    armor::BodyPart,
    injuries::{
        GainedInjury, InflictedInjuries, InjuryName, InspectText, LogText, PopupText, RolledInjury,
    },
    severity::Severity,
};

/// Message: an injury was inflicted on a target.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct InjuryInflicted {
    /// Target entity.
    pub target:       Entity,
    /// Injury to record.
    pub gained:       GainedInjury,
    /// Display name.
    pub name:         InjuryName,
    /// Body part.
    pub part:         BodyPart,
    /// Severity.
    pub severity:     Severity,
    /// Popup text.
    pub popup_text:   PopupText,
    /// Log text.
    pub log_text:     LogText,
    /// Inspect text.
    pub inspect_text: InspectText,
}

impl InjuryInflicted {
    /// Build from a rolled injury table result.
    #[must_use]
    pub fn from_rolled(target: Entity, rolled: RolledInjury) -> Self {
        let name = rolled.name.clone();
        let part = rolled.part;
        let severity = rolled.severity;
        let popup_text = rolled.popup_text.clone();
        let log_text = rolled.log_text.clone();
        let inspect_text = rolled.inspect_text.clone();
        Self {
            target,
            gained: rolled.into_gained(),
            name,
            part,
            severity,
            popup_text,
            log_text,
            inspect_text,
        }
    }
}

/// System: record injuries and refresh bleed state.
pub fn apply_injury(
    mut inflicted: MessageReader<InjuryInflicted>,
    mut ledgers: Query<&mut InflictedInjuries>,
    mut commands: Commands,
) {
    for message in inflicted.read() {
        let target = message.target;
        let Ok(mut entity) = commands.get_entity(target) else {
            continue;
        };

        let bleed = if let Ok(mut ledger) = ledgers.get_mut(target) {
            ledger.gain(message.gained.clone());
            ledger.bleed()
        } else {
            let mut ledger = InflictedInjuries::default();
            ledger.gain(message.gained.clone());
            let bleed = ledger.bleed();
            entity.insert(ledger);
            bleed
        };

        entity.insert(bleed);
    }
}
