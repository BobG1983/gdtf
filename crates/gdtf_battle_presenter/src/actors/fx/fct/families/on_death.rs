//! The ON-DEATH consequence family (GTW-547, palette-ised in GTW-572): the transient BOLD
//! `"BOOM"` blast marker at every cell where an on-death effect fanned, off the sim's
//! [`OnDeathOccurred`](gdtf_battle_sim::OnDeathOccurred) message.
//!
//! It closes the Explode VISIBILITY gap: the sim's
//! [`Explode`](gdtf_battle_sim::OnDeathEffect::Explode) applies its blast as a direct,
//! RNG-free HP drain that rides NO shot-impact FX nor attrition pop, so without this marker
//! a detonation would be invisible. Drawn the lethal
//! [`FctValence::Lethal`](super::super::palette::FctValence::Lethal) blood-red at
//! [`FctEmphasis::Bold`](super::super::text::FctEmphasis::Bold) — the one BOLD family in the
//! palette (a terminal death, not a recurring tick). The message carries the death cell —
//! [`PopAnchor::Carried`]; a cover death carries [`Entity::PLACEHOLDER`](bevy::prelude::Entity)
//! and still has a valid cell, so the marker never depends on the entity.

use gdtf_battle_sim::OnDeathOccurred;

use super::super::{
    palette::{FctValence, valence_color},
    pop::{ConsequenceFct, ConsequencePop, PopAnchor},
    text::CombatText,
};

/// The blast-marker string every on-death pop renders — a short, all-caps genre
/// "detonation" tag so an on-death effect (the Explode blast especially) reads at its
/// origin cell. Framework plumbing (a literal fed into a [`CombatText`], not a domain
/// quantity — the message carries no per-cell number to render).
const ON_DEATH_MARKER: &str = "BOOM";

/// The on-death family marker — `OnDeathOccurred` → a BOLD lethal-red `"BOOM"` blast tag.
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
    use gdtf_battle_sim::{Cell, CellLevel, Level, OnDeathOccurred};

    use super::{
        super::super::{pop::ConsequenceFct, text::FctEmphasis},
        FctValence, ON_DEATH_MARKER, OnDeathFct, PopAnchor, valence_color,
    };

    /// An on-death occurrence classifies to the blast marker drawn BOLD in the lethal
    /// blood-red, anchored at the message's carried death cell — the terminal-death
    /// detonation tag, the one bold family in the palette.
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

    /// A COVER death carries `Entity::PLACEHOLDER` (cover is not an entity) but a valid
    /// cell — the classify still anchors the marker at that cell, never at the entity.
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
