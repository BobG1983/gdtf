use gdtf_battle_sim::effects::bleed::Bleeding;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

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

                #[test]
    fn a_bleeding_classifies_to_an_amber_bleeding_tag_on_the_ganger() {
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
            "the bleeding pop anchors on the ganger (drawn cell, resolved by the reader)",
        );
    }
}
