//! Armor-broken floating combat text.

use gdtf_battle_sim::armor_wear::ArmorBroken;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

/// Consequence family for [`ArmorBroken`].
#[derive(Debug, Clone, Copy)]
pub struct ArmorBrokenFct;

impl ConsequenceFct for ArmorBrokenFct {
    type Signal = ArmorBroken;

    fn classify(signal: &Self::Signal) -> ConsequencePop {
        ConsequencePop::new(
            CombatText::new("Armor Broken"),
            valence_color(FctValence::Damage),
            PopAnchor::GangerPosition(signal.ganger),
        )
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::Entity;
    use gdtf_battle_sim::{armor::BodyPart, armor_wear::ArmorBroken};

    use super::{
        super::super::pop::ConsequenceFct, ArmorBrokenFct, FctValence, PopAnchor, valence_color,
    };

    #[test]
    fn an_armor_broken_classifies_to_a_red_armor_broken_tag() {
        let ganger = Entity::PLACEHOLDER;
        let pop = ArmorBrokenFct::classify(&ArmorBroken::new(ganger, BodyPart::Torso));
        assert_eq!(
            &**pop.text(),
            "Armor Broken",
            "the armor-broken consequence pops the \"Armor Broken\" tag",
        );
        assert_eq!(
            pop.color(),
            valence_color(FctValence::Damage),
            "the armor-broken pop is drawn the damage RED (the destroy crossing reads heavier)",
        );
        assert_ne!(
            pop.color(),
            valence_color(FctValence::Status),
            "the armor-broken RED valence must differ from the wound AMBER",
        );
        assert_eq!(
            pop.anchor(),
            PopAnchor::GangerPosition(ganger),
            "the armor-broken pop anchors on the ganger (drawn cell, resolved by the reader)",
        );
    }
}
