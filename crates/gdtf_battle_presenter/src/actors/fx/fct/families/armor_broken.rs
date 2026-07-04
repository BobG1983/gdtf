//! The ARMOR-BROKEN consequence family (GTW-302 slice 4, palette-ised in GTW-572): the RED
//! `"Armor Broken"` tag for a worn piece that crossed from protecting to useless, off the
//! sim's [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) crossing message.
//!
//! The destroy CROSSING reads heavier than ordinary wear, so it pops the damage RED (the
//! contract's "AMBER/RED", drawn the redder of the two), alongside the `read_armor_broken`
//! spark flash. It anchors at the ganger's live position ([`PopAnchor::GangerPosition`],
//! fail-closed). The numeric `"Armor -N"` variant stays DEFERRED: [`ArmorBroken`] carries no
//! integrity-delta amount (only the `{ ganger, part }` crossing).

use gdtf_battle_sim::ArmorBroken;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

/// The armor-broken family marker — `ArmorBroken` → a RED `"Armor Broken"` tag.
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
    use gdtf_battle_sim::{ArmorBroken, BodyPart};

    use super::{
        super::super::pop::ConsequenceFct, ArmorBrokenFct, FctValence, PopAnchor, valence_color,
    };

    /// An `ArmorBroken` consequence classifies to the RED `"Armor Broken"` pop — the destroy
    /// crossing reads heavier than ordinary wear, distinct from the AMBER wound family — and
    /// anchors at the ganger's live position.
    #[test]
    fn an_armor_broken_classifies_to_a_red_armor_broken_tag() {
        // The classify never dereferences the entity — a placeholder handle drives the path.
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
            valence_color(FctValence::Wound),
            "the armor-broken RED valence must differ from the wound AMBER",
        );
        assert_eq!(
            pop.anchor(),
            PopAnchor::GangerPosition(ganger),
            "the armor-broken pop anchors at the ganger's live position",
        );
    }
}
