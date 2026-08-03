use super::{InjuryEffect, InjuryName, InspectText, LogText, PopupText};
use crate::{armor::BodyPart, severity::Severity};

#[derive(Debug, Clone, PartialEq)]
pub struct RolledInjury {
        pub name:         InjuryName,
        pub part:         BodyPart,
        pub severity:     Severity,
        pub effects:      Vec<InjuryEffect>,
        pub popup_text:   PopupText,
        pub log_text:     LogText,
        pub inspect_text: InspectText,
}

impl RolledInjury {
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

#[derive(Debug, Clone, PartialEq)]
pub struct GainedInjury {
        pub name:         InjuryName,
        pub part:         BodyPart,
        pub severity:     Severity,
        pub effects:      Vec<InjuryEffect>,
        pub inspect_text: InspectText,
}

impl GainedInjury {
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
