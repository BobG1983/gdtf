//! Injury floating combat text from authored popup text.

use gdtf_battle_sim::acts::InjuryInflicted;

use super::super::{
    palette::severity_color,
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

/// Consequence family for [`InjuryInflicted`].
#[derive(Debug, Clone, Copy)]
pub struct InjuryFct;

impl ConsequenceFct for InjuryFct {
    type Signal = InjuryInflicted;

    fn classify(signal: &Self::Signal) -> ConsequencePop {
        ConsequencePop::new(
            CombatText::new((*signal.popup_text).clone()),
            severity_color(signal.severity),
            PopAnchor::GangerPosition(signal.target),
        )
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::Entity;
    use gdtf_battle_sim::{
        acts::InjuryInflicted,
        armor::BodyPart,
        injuries::{GainedInjury, InjuryName, InspectText, LogText, PopupText},
        severity::Severity,
    };

    use super::{super::super::pop::ConsequenceFct, InjuryFct, PopAnchor, severity_color};

    fn injury(popup: &str, severity: Severity) -> InjuryInflicted {
        let name = InjuryName::new("Lost Eye".to_owned());
        InjuryInflicted {
            target: Entity::PLACEHOLDER,
            gained: GainedInjury::new(
                name.clone(),
                BodyPart::Head,
                severity,
                Vec::new(),
                InspectText::new("Lost Eye -- -2 Aim".to_owned()),
            ),
            name,
            part: BodyPart::Head,
            severity,
            popup_text: PopupText::new(popup.to_owned()),
            log_text: LogText::new("loses an eye".to_owned()),
            inspect_text: InspectText::new("Lost Eye -- -2 Aim".to_owned()),
        }
    }

    #[test]
    fn an_injury_pop_renders_the_popup_text_in_the_severity_color() {
        let pop = InjuryFct::classify(&injury("LOST EYE", Severity::Critical));
        assert_eq!(
            &**pop.text(),
            "LOST EYE",
            "the injury pop renders the authored popup_text verbatim",
        );
        assert_eq!(
            pop.color(),
            severity_color(Severity::Critical),
            "the injury pop is drawn the severity-scaled wound valence",
        );
        assert_eq!(
            pop.anchor(),
            PopAnchor::GangerPosition(Entity::PLACEHOLDER),
            "the injury pop anchors on the wounded ganger (drawn cell, resolved by the reader)",
        );
    }

    #[test]
    fn the_injury_valence_scales_with_severity() {
        let minor = InjuryFct::classify(&injury("BRUISE", Severity::Minor));
        let critical = InjuryFct::classify(&injury("LOST EYE", Severity::Critical));
        assert_eq!(minor.color(), severity_color(Severity::Minor));
        assert_eq!(critical.color(), severity_color(Severity::Critical));
        assert_ne!(
            minor.color(),
            critical.color(),
            "a Critical injury must read a hotter swatch than a Minor one (valence by severity)",
        );
    }
}
