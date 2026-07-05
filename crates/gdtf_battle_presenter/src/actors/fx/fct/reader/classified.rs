//! The [`ClassifiedPop`] value type — one ready-to-spawn floating-combat-text pop.

use bevy::prelude::*;

use super::super::text::{CombatText, FctEmphasis};

/// One ready-to-spawn floating-combat-text pop — the classified string, its valence color,
/// and its emphasis weight, before it is anchored at the hit cell and given its stack slot.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color, FctEmphasis)` tuple):
/// [`classify_report`](super::classify::classify_report) builds the ordered list of pops one
/// round's [`HitReport`](gdtf_battle_sim::HitReport) yields;
/// [`spawn_shot_projectiles`](super::super::super::spawn_shot_projectiles) threads that list
/// THROUGH the staggered projectile pipeline and
/// [`animate_impact`](super::super::super::animate_impact) anchors each at the round's cell with
/// the next [`FctStackIndex`](super::super::text::FctStackIndex) when the impact lands. The
/// [`Color`](bevy::prelude::Color) is framework plumbing (the swatch fed straight to the
/// primitive), the only bare type the no-bare-types rule permits here.
///
/// `pub(in crate::actors::fx)`: built here by the classifier, consumed by the sibling
/// `projectile` / `impact` modules — read through the [`text`](Self::text) /
/// [`color`](Self::color) / [`emphasis`](Self::emphasis) accessors (the fields stay
/// `pub(super)` — the reader subtree — so a pop is only constructed through the classifier,
/// never field-assembled outside the reader module).
#[derive(Debug, Clone)]
pub(in crate::actors::fx) struct ClassifiedPop {
    /// The combat-text string this pop renders (a damage number, a wound tag, `"Grazed"`, …).
    pub(super) text:     CombatText,
    /// The valence swatch the pop is drawn in (the damage RED / wound AMBER / neutral GREY /
    /// lethal RED family the event maps to).
    pub(super) color:    Color,
    /// The styling weight the pop is drawn at — [`FctEmphasis::Bold`] for the lethal
    /// DOWN / DEAD tag (the contract's "RED bold"), [`FctEmphasis::Normal`] for every other.
    pub(super) emphasis: FctEmphasis,
}

impl ClassifiedPop {
    /// Build an ordinary (body-weight) classified pop from its text + valence color.
    pub(super) const fn new(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Normal,
        }
    }

    /// Build an EMPHASIZED (bold) classified pop from its text + valence color — the lethal
    /// DOWN / DEAD tag, drawn the heaviest in the blood family per the contract's "RED bold".
    pub(super) const fn new_bold(text: CombatText, color: Color) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Bold,
        }
    }

    /// The pop's combat-text string (consumed by reference at spawn — the caller clones the
    /// inner [`CombatText`] into the [`Text2d`](bevy::prelude::Text2d)).
    pub(in crate::actors::fx) const fn text(&self) -> &CombatText {
        &self.text
    }

    /// The pop's valence swatch — the [`Color`](bevy::prelude::Color) fed straight to
    /// [`spawn_floating_text`](super::super::text::spawn_floating_text).
    pub(in crate::actors::fx) const fn color(&self) -> Color {
        self.color
    }

    /// The pop's styling weight ([`FctEmphasis::Bold`] for the lethal tag, else
    /// [`FctEmphasis::Normal`]).
    pub(in crate::actors::fx) const fn emphasis(&self) -> FctEmphasis {
        self.emphasis
    }
}
