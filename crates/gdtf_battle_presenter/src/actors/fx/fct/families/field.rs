use gdtf_battle_sim::effects::fields::FieldTicked;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

#[derive(Debug, Clone, Copy)]
pub struct FieldFct;

impl ConsequenceFct for FieldFct {
    type Signal = FieldTicked;

    fn classify(signal: &Self::Signal) -> ConsequencePop {
        ConsequencePop::new(
            CombatText::new(format!("-{}", *signal.amount)),
            valence_color(FctValence::Field),
            PopAnchor::Carried(signal.at),
        )
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::Entity;
    use gdtf_battle_sim::{
        effects::fields::{FieldDamage, FieldTicked},
        prelude::{Cell, CellLevel, Level},
    };

    use super::{
        super::super::pop::ConsequenceFct, FctValence, FieldFct, PopAnchor, valence_color,
    };

            #[test]
    fn a_field_tick_classifies_to_a_hazard_minus_amount_tag() {
        let at = CellLevel::new(Cell::new(8, 5), Level::new(0));
        let pop = FieldFct::classify(&FieldTicked::new(
            Entity::PLACEHOLDER,
            at,
            FieldDamage::new(3),
        ));
        assert_eq!(
            &**pop.text(),
            "-3",
            "the field tick pops the drained HP as a \"-N\" number",
        );
        assert_eq!(
            pop.color(),
            valence_color(FctValence::Field),
            "the field tick pop is drawn the hazard Field orange valence",
        );
        assert_eq!(
            pop.anchor(),
            PopAnchor::Carried(at),
            "the field tick pop anchors at the message's carried cell",
        );
    }
}
