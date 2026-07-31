//! The INJURY consequence family (GTW-439 C1, palette-ised in GTW-572): the transient flash
//! for a freshly-inflicted named injury, off the GTW-438
//! [`InjuryInflicted`](gdtf_battle_sim::acts::InjuryInflicted) boundary message.
//!
//! The pop renders the authored `popup_text` verbatim in a VALENCE BY SEVERITY — the
//! [`severity_color`](super::super::palette::severity_color) wound-family amber ramp scaled
//! by the rolled tier (a worse injury reads hotter; `Minor` the light amber base, `Critical`
//! the hot orange-red one shy of the lethal red) — NOT a flat
//! [`valence_color`](super::super::palette::valence_color) swatch: the injury family is the
//! one palette member riding the ramp. It anchors at the cell the wounded ganger is DRAWN at
//! ([`PopAnchor::GangerPosition`], fail-closed — the reader resolves it from the
//! [`DrawnPosition`](crate::DrawnPosition) mirror, not the position the sim has already run
//! ahead to, GTW-889). The transient flash is the message's ONLY
//! presenter job: the durable per-ganger injury LIST is the
//! [`InflictedInjuries`](gdtf_battle_sim::injuries::InflictedInjuries) ledger in the inspect panel.

use gdtf_battle_sim::acts::InjuryInflicted;

use super::super::{
    palette::severity_color,
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

/// The injury family marker — `InjuryInflicted` → its `popup_text` in the severity ramp.
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

    /// An [`InjuryInflicted`] carrying `popup` at `severity` for a placeholder target — the
    /// classify reads only `target` / `popup_text` / `severity`; the ledger + log / inspect
    /// texts are self-consistent filler.
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

    /// The injury FCT pop renders the authored `popup_text` verbatim, draws it in the
    /// `severity_color` ramp scaled by the rolled tier (C1: valence BY SEVERITY), and
    /// anchors on the wounded ganger (the reader resolves that to its DRAWN cell).
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

    /// PIN-DISCRIMINATING — the valence tracks the rolled tier: a WORSE injury reads a
    /// DIFFERENT (hotter) swatch than a milder one, so the mapping is genuinely
    /// severity-scaled, not a flat constant color.
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
