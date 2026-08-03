use gdtf_battle_sim::suppression::SuppressionApplied;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

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
