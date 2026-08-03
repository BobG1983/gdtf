use gdtf_battle_sim::effects::dot::DotTicked;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

#[derive(Debug, Clone, Copy)]
pub struct DotFct;

impl ConsequenceFct for DotFct {
    type Signal = DotTicked;

    fn classify(signal: &Self::Signal) -> ConsequencePop {
        ConsequencePop::new(
            CombatText::new(format!("-{}", *signal.amount)),
            valence_color(FctValence::Dot),
            PopAnchor::Carried(signal.at),
        )
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::Entity;
    use gdtf_battle_sim::{
        effects::dot::DotTicked,
        prelude::{Cell, CellLevel, Level},
        weapon::DotDamage,
    };

    use super::{super::super::pop::ConsequenceFct, DotFct, FctValence, PopAnchor, valence_color};

                #[test]
    fn a_dot_tick_classifies_to_a_toxic_minus_amount_tag() {
        let at = CellLevel::new(Cell::new(7, 9), Level::new(0));
        let pop = DotFct::classify(&DotTicked::new(Entity::PLACEHOLDER, at, DotDamage::new(4)));
        assert_eq!(
            &**pop.text(),
            "-4",
            "the DOT tick pops the drained HP as a \"-N\" number",
        );
        assert_eq!(
            pop.color(),
            valence_color(FctValence::Dot),
            "the DOT tick pop is drawn the toxic Dot green valence",
        );
        assert_eq!(
            pop.anchor(),
            PopAnchor::Carried(at),
            "the DOT tick pop anchors at the message's carried cell",
        );
    }
}
