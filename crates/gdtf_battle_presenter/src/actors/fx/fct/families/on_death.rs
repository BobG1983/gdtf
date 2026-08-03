use gdtf_battle_sim::effects::on_death::OnDeathOccurred;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

const ON_DEATH_MARKER: &str = "BOOM";

#[derive(Debug, Clone, Copy)]
pub struct OnDeathFct;

impl ConsequenceFct for OnDeathFct {
    type Signal = OnDeathOccurred;

    fn classify(signal: &Self::Signal) -> ConsequencePop {
        ConsequencePop::new_bold(
            CombatText::new(ON_DEATH_MARKER),
            valence_color(FctValence::Lethal),
            PopAnchor::Carried(signal.at),
        )
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::Entity;
    use gdtf_battle_sim::{
        effects::on_death::OnDeathOccurred,
        prelude::{Cell, CellLevel, Level},
    };

    use super::{
        super::super::{pop::ConsequenceFct, text::FctEmphasis},
        FctValence, ON_DEATH_MARKER, OnDeathFct, PopAnchor, valence_color,
    };

                #[test]
    fn an_on_death_classifies_to_a_bold_lethal_blast_marker() {
        let at = CellLevel::new(Cell::new(11, 4), Level::new(0));
        let pop = OnDeathFct::classify(&OnDeathOccurred::new(Entity::PLACEHOLDER, at));
        assert_eq!(
            &**pop.text(),
            ON_DEATH_MARKER,
            "the on-death pop renders the blast marker tag",
        );
        assert_eq!(
            pop.color(),
            valence_color(FctValence::Lethal),
            "the on-death pop is drawn the lethal blood-red valence (a terminal death)",
        );
        assert_eq!(
            pop.emphasis(),
            FctEmphasis::Bold,
            "the on-death marker is the one BOLD family in the palette",
        );
        assert_eq!(
            pop.anchor(),
            PopAnchor::Carried(at),
            "the on-death marker anchors at the message's carried death cell",
        );
    }

            #[test]
    fn a_cover_on_death_still_anchors_at_the_cover_cell() {
        let at = CellLevel::new(Cell::new(3, 12), Level::new(0));
        let pop = OnDeathFct::classify(&OnDeathOccurred::cover(at));
        assert_eq!(
            pop.anchor(),
            PopAnchor::Carried(at),
            "a cover on-death (placeholder entity) still anchors at the cover cell",
        );
    }
}
