//! Result of an injury roll and the form stored on a combatant.

use super::{InjuryEffect, InjuryName, InspectText, LogText, PopupText};
use crate::{armor::BodyPart, severity::Severity};

/// Full result of a successful injury roll (includes presentation text).
#[derive(Debug, Clone, PartialEq)]
pub struct RolledInjury {
    /// Injury name key.
    pub name:         InjuryName,
    /// Body part that was hit.
    pub part:         BodyPart,
    /// Severity tier.
    pub severity:     Severity,
    /// Effects to apply.
    pub effects:      Vec<InjuryEffect>,
    /// Popup text.
    pub popup_text:   PopupText,
    /// Log text.
    pub log_text:     LogText,
    /// Inspect text.
    pub inspect_text: InspectText,
}

impl RolledInjury {
    /// Build a rolled injury.
    #[must_use]
    pub const fn new(
        name: InjuryName,
        part: BodyPart,
        severity: Severity,
        effects: Vec<InjuryEffect>,
        popup_text: PopupText,
        log_text: LogText,
        inspect_text: InspectText,
    ) -> Self {
        Self {
            name,
            part,
            severity,
            effects,
            popup_text,
            log_text,
            inspect_text,
        }
    }

    /// Convert into the form stored on the combatant (drops popup/log text).
    #[must_use]
    pub fn into_gained(self) -> GainedInjury {
        GainedInjury::new(
            self.name,
            self.part,
            self.severity,
            self.effects,
            self.inspect_text,
        )
    }
}

/// Injury that has been applied to a combatant.
#[derive(Debug, Clone, PartialEq)]
pub struct GainedInjury {
    /// Injury name key.
    pub name:         InjuryName,
    /// Body part.
    pub part:         BodyPart,
    /// Severity.
    pub severity:     Severity,
    /// Active effects.
    pub effects:      Vec<InjuryEffect>,
    /// Inspect text.
    pub inspect_text: InspectText,
}

impl GainedInjury {
    /// Build a gained injury.
    #[must_use]
    pub const fn new(
        name: InjuryName,
        part: BodyPart,
        severity: Severity,
        effects: Vec<InjuryEffect>,
        inspect_text: InspectText,
    ) -> Self {
        Self {
            name,
            part,
            severity,
            effects,
            inspect_text,
        }
    }
}
