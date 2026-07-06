//! The DAMAGE-OVER-TIME consequence family (GTW-544, palette-ised in GTW-572): the
//! transient `"-N"` attrition pop for a ganger a burning / caustic DOT drained this round,
//! off the sim's [`DotTicked`](gdtf_battle_sim::effects::dot::DotTicked) per-round message.
//!
//! Drawn the toxic [`FctValence::Dot`](super::super::palette::FctValence::Dot) green — its
//! OWN recurring-attrition valence, distinct from a fresh weapon hit (RED) or a bleed
//! status tag (AMBER). The message carries the drained cell, so the anchor is
//! [`PopAnchor::Carried`]. The persistent DOT state is the sim's
//! [`Dot`](gdtf_battle_sim::weapon::Dot) affliction, NOT this one-shot pop.

use gdtf_battle_sim::effects::dot::DotTicked;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

/// The DOT family marker — `DotTicked` → a toxic-green `"-N"` attrition number.
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

    /// A DOT tick classifies to a `"-N"` drained-HP pop drawn in the toxic Dot green,
    /// anchored at the message's carried cell — the amount is rendered as a familiar
    /// `-{hp}` number, in the DOT attrition valence.
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
