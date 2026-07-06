//! The BLEEDING consequence family (GTW-302 slice 4, palette-ised in GTW-572): the AMBER
//! `"Bleeding"` status tag for a ganger the bleed-out clock drained this round, off the
//! sim's per-tick [`Bleeding`](gdtf_battle_sim::effects::bleed::Bleeding) message.
//!
//! The pop sits ALONGSIDE the `read_bleeding` blood flash (the flash is the splash, this is
//! the labelled tag) and anchors at the ganger's live position — the message carries no
//! cell, so the anchor is [`PopAnchor::GangerPosition`] (fail-closed: a `Position`-less
//! ganger pops nothing). The flat wound AMBER (not the severity ramp) because the message
//! carries no severity to ramp by.

use gdtf_battle_sim::effects::bleed::Bleeding;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

/// The bleeding family marker — `Bleeding` → an AMBER `"Bleeding"` tag over the ganger.
#[derive(Debug, Clone, Copy)]
pub struct BleedingFct;

impl ConsequenceFct for BleedingFct {
    type Signal = Bleeding;

    fn classify(signal: &Self::Signal) -> ConsequencePop {
        ConsequencePop::new(
            CombatText::new("Bleeding"),
            valence_color(FctValence::Status),
            PopAnchor::GangerPosition(signal.ganger),
        )
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::Entity;
    use gdtf_battle_sim::effects::bleed::Bleeding;

    use super::{
        super::super::pop::ConsequenceFct, BleedingFct, FctValence, PopAnchor, valence_color,
    };

    /// A `Bleeding` consequence classifies to the AMBER `"Bleeding"` status pop (the flat
    /// wound/status swatch — no severity to ramp by), anchored at the ganger's live
    /// position (fail-closed in the reader when the ganger has none).
    #[test]
    fn a_bleeding_classifies_to_an_amber_bleeding_tag_on_the_ganger() {
        // The classify never dereferences the entity — a placeholder handle drives the path.
        let ganger = Entity::PLACEHOLDER;
        let pop = BleedingFct::classify(&Bleeding::new(ganger));
        assert_eq!(
            &**pop.text(),
            "Bleeding",
            "the bleeding consequence pops the \"Bleeding\" tag",
        );
        assert_eq!(
            pop.color(),
            valence_color(FctValence::Status),
            "the bleeding pop is drawn the AMBER wound/status valence",
        );
        assert_eq!(
            pop.anchor(),
            PopAnchor::GangerPosition(ganger),
            "the bleeding pop anchors at the ganger's live position",
        );
    }
}
