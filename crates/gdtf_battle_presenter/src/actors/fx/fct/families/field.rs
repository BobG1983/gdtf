//! The AREA-DAMAGE-FIELD consequence family (GTW-545, palette-ised in GTW-572): the
//! transient `"-N"` attrition pop for a ganger a persistent damage ZONE drained this round,
//! off the sim's [`FieldTicked`](gdtf_battle_sim::effects::fields::FieldTicked) per-round message.
//!
//! Drawn the hazard [`FctValence::Field`](super::super::palette::FctValence::Field) orange —
//! its OWN environmental-attrition valence, distinct from a fresh weapon hit (RED) and from
//! the DOT's toxic green (a field is a ZONE you stand in, not an affliction you carry). The
//! message carries the field cell, so the anchor is [`PopAnchor::Carried`]. The persistent
//! ZONE is the sim's [`FieldRegistry`](gdtf_battle_sim::effects::fields::FieldRegistry), drawn by
//! [`draw_field_overlay`](crate::draw_field_overlay), NOT this one-shot pop.

use gdtf_battle_sim::effects::fields::FieldTicked;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

/// The field family marker — `FieldTicked` → a hazard-orange `"-N"` attrition number.
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

    /// A field tick classifies to a `"-N"` drained-HP pop drawn in the hazard Field orange,
    /// anchored at the message's carried field cell.
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
