//! The SUPPRESSION consequence family (GTW-526 C8, palette-ised in GTW-572): the transient
//! `"SUPPRESSED"` pop for a ganger freshly pinned down, off the sim's
//! [`SuppressionApplied`](gdtf_battle_sim::suppression::SuppressionApplied) message.
//!
//! Drawn the cowed [`FctValence::Suppressed`](super::super::palette::FctValence::Suppressed)
//! blue-grey — the same colour-drained family the suppressed sprite tint uses, so the
//! transient pop and the persistent desaturation read as ONE signal. The message carries the
//! pinned cell, so the anchor is [`PopAnchor::Carried`] (no `Position` lookup). The pop is a
//! MOMENT signal; the persistent state is the sprite tint (`reframe_ganger_sprites`).

use gdtf_battle_sim::suppression::SuppressionApplied;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

/// The suppression family marker — `SuppressionApplied` → a cowed `"SUPPRESSED"` tag.
#[derive(Debug, Clone, Copy)]
pub struct SuppressionFct;

impl ConsequenceFct for SuppressionFct {
    type Signal = SuppressionApplied;

    fn classify(signal: &Self::Signal) -> ConsequencePop {
        ConsequencePop::new(
            CombatText::new("SUPPRESSED"),
            valence_color(FctValence::Suppressed),
            PopAnchor::Carried(signal.at),
        )
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::Entity;
    use gdtf_battle_sim::{
        prelude::{Cell, CellLevel, Level},
        suppression::SuppressionApplied,
    };

    use super::{
        super::super::pop::ConsequenceFct, FctValence, PopAnchor, SuppressionFct, valence_color,
    };

    /// A suppression event classifies to the cowed `"SUPPRESSED"` blue-grey pop, anchored
    /// at the message's carried pinned cell (C8: the moment-pop signal).
    #[test]
    fn a_suppression_classifies_to_a_cowed_suppressed_tag_at_the_cell() {
        let at = CellLevel::new(Cell::new(11, 4), Level::new(0));
        let pop = SuppressionFct::classify(&SuppressionApplied::new(Entity::PLACEHOLDER, at));
        assert_eq!(
            &**pop.text(),
            "SUPPRESSED",
            "the suppression consequence pops the \"SUPPRESSED\" tag",
        );
        assert_eq!(
            pop.color(),
            valence_color(FctValence::Suppressed),
            "the suppression pop is drawn the cowed Suppressed blue-grey valence",
        );
        assert_eq!(
            pop.anchor(),
            PopAnchor::Carried(at),
            "the suppression pop anchors at the message's carried pinned cell",
        );
    }
}
