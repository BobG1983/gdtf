//! GTW-544 (child GTW-41e): the DAMAGE-OVER-TIME floating-combat-text reader — the transient
//! `"-N"` attrition pop for a ganger a burning / caustic DOT drained this round, routed off the
//! sim's [`DotTicked`](gdtf_battle_sim::DotTicked) per-round message.
//!
//! Each round the sim's [`tick_dot`](gdtf_battle_sim::tick_dot) clock drains a flat
//! [`amount`](gdtf_battle_sim::DotTicked::amount) of an afflicted ganger's HP and emits one
//! [`DotTicked`](gdtf_battle_sim::DotTicked) carrying the ganger, its `(cell, level)`, and the HP
//! drained. This module routes that to a rise-and-fade FCT pop reading `"-N"` (the drained HP)
//! over the ganger's cell, drawn in the toxic [`FctValence::Dot`](super::palette::FctValence::Dot)
//! green — its OWN attrition valence, distinct from the raw-hit damage RED, the wound AMBER, the
//! bleed status tag, the neutral GREY, the lethal RED, and the cowed suppression blue-grey — so a
//! recurring DOT tick reads as its own signal rather than being mistaken for a fresh weapon hit.
//!
//! It MIRRORS the [`Bleeding`](gdtf_battle_sim::Bleeding) FCT-pop precedent
//! ([`read_consequence_fct`](super::read_consequence_fct)) and the suppression reader
//! ([`read_suppression_fct`](super::read_suppression_fct)): it resolves the drained cell straight
//! off the message's [`at`](gdtf_battle_sim::DotTicked::at) (no `Position` lookup — the message
//! already carries the cell), and stacks simultaneous same-cell pops via an
//! [`FctStackIndex`](super::text::FctStackIndex) counter so two DOT ticks on one cell this round
//! fan out vertically instead of overlapping. The classify half is the pure, World-free
//! [`dot_tick_pop`] so the tag + valence mapping is unit-testable with no [`App`](bevy::prelude::App).
//!
//! Pure VIEW (ADR-0001): it only READS the message, then SPAWNS a presenter pop; it never polls
//! raw sim state and never writes the sim.

use std::collections::HashMap;

use bevy::prelude::*;
use gdtf_battle_sim::{DotDamage, DotTicked};

use super::{
    super::FxTuning,
    palette::{FctValence, valence_color},
    text::{CombatText, FctEmphasis, FctStackIndex, spawn_floating_text},
};

/// One ready-to-spawn DOT-tick pop — the `"-N"` drained-HP string + its toxic attrition color,
/// before it is anchored at the ganger's cell and given its stack slot.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color)` tuple), mirroring the slice-3
/// `ClassifiedPop` / slice-4 `AuxPop` / suppression `SuppressionPop` shape: [`dot_tick_pop`]
/// builds it, and [`read_dot_fct`] anchors it. The [`Color`](bevy::prelude::Color) is framework
/// plumbing (the swatch fed straight to the primitive), the only bare type the no-bare-types rule
/// permits here.
#[derive(Debug, Clone)]
pub(in crate::actors::fx) struct DotTickPop {
    /// The combat-text string this pop renders — the drained HP as a `"-N"` number.
    text:  CombatText,
    /// The valence swatch the pop is drawn in — the toxic
    /// [`FctValence::Dot`](super::palette::FctValence::Dot) green.
    color: Color,
}

/// Classify a DOT tick of `amount` HP into the FCT pop it yields — the pure, World-free mapping
/// (GTW-544).
///
/// The pop renders the drained HP as a `"-N"` number (the same `-{hp}` shape the per-shot damage
/// pop uses, so a DOT tick reads as a familiar HP-loss number) drawn in the toxic
/// [`FctValence::Dot`](super::palette::FctValence::Dot) green — its own recurring-attrition valence,
/// distinct from every other FCT valence. Split out of [`read_dot_fct`] so the tag + valence
/// mapping is unit-testable with no [`App`](bevy::prelude::App).
#[must_use]
pub(in crate::actors::fx) fn dot_tick_pop(amount: DotDamage) -> DotTickPop {
    DotTickPop {
        text:  CombatText::new(format!("-{}", *amount)),
        color: valence_color(FctValence::Dot),
    }
}

/// `Update` (`PresenterSystems::Draw`): drain [`DotTicked`](gdtf_battle_sim::DotTicked) and spawn
/// one `"-N"` floating-combat-text pop per afflicted ganger's per-round drain (GTW-544).
///
/// For each drained message it:
///
/// 1. Classifies the pop via [`dot_tick_pop`] — the drained HP as a `"-N"` number drawn in the
///    toxic [`FctValence::Dot`](super::palette::FctValence::Dot) green.
/// 2. Reads the drained `(cell, level)` straight off the message's
///    [`at`](gdtf_battle_sim::DotTicked::at) [`CellLevel`](gdtf_battle_sim::CellLevel), via
///    the canonical [`CellLevel::split`](gdtf_battle_sim::CellLevel::split) decompose (GTW-565
///    — the message already carries the cell, so no `Position` lookup is needed).
/// 3. Spawns the pop via [`spawn_floating_text`] at the next per-cell [`FctStackIndex`] (so two
///    DOT ticks on one cell this round fan out vertically), at body weight
///    ([`FctEmphasis::Normal`] — bold is reserved for the lethal DOWN / DEAD tag), with the
///    hot-reloadable [`FxTuning`] lifetime + rise (GTW-327).
///
/// The pop is the message's ONLY presenter job (the moment the tick lands): the persistent DOT
/// state lives on the sim's [`Dot`](gdtf_battle_sim::Dot) affliction, NOT this transient flash.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the
/// [`MessageReader`](bevy::ecs::message::MessageReader), and [`Res<FxTuning>`] for the
/// hot-reloadable pop lifetime + rise. Its plugin gate adds `resource_exists::<FxTuning>` + the
/// message buffer so the params are always valid (`bevy-traps.md` #1 / #4). It never writes the
/// sim.
pub fn read_dot_fct(
    mut commands: Commands,
    mut ticked: MessageReader<DotTicked>,
    tuning: Res<FxTuning>,
) {
    // The per-cell stack counter for THIS drain, keyed by the integer cell + level so two DOT
    // ticks on one cell fan one step further down each and distinct cells never share a slot
    // (the consequence / suppression reader's same-cell stacking precedent).
    let mut stacks: HashMap<(i32, i32, u8), usize> = HashMap::new();

    for message in ticked.read() {
        let pop = dot_tick_pop(message.amount);

        // The drained cell is carried directly on the message (the sim resolved the ganger's
        // Position at tick time) — the canonical CellLevel::split decompose (GTW-565); no
        // Position lookup, and no fail-closed branch (the message always carries a valid key).
        let (cell, level) = message.at.split();

        let slot = stacks.entry((cell.x, cell.y, *level)).or_insert(0);
        spawn_floating_text(
            &mut commands,
            pop.text,
            pop.color,
            FctEmphasis::Normal,
            cell,
            level,
            FctStackIndex::new(*slot),
            tuning.fct_ttl_seconds,
            tuning.fct_rise_rate,
        );
        *slot += 1;
    }
}

#[cfg(test)]
mod test {
    use gdtf_battle_sim::DotDamage;

    use super::{FctValence, dot_tick_pop, valence_color};

    /// A DOT tick classifies to a `"-N"` drained-HP pop drawn in the toxic Dot green — the amount
    /// is rendered as a familiar `-{hp}` number, in the DOT attrition valence.
    #[test]
    fn a_dot_tick_classifies_to_a_toxic_minus_amount_tag() {
        let pop = dot_tick_pop(DotDamage::new(4));
        assert_eq!(
            &*pop.text, "-4",
            "the DOT tick pops the drained HP as a \"-N\" number",
        );
        assert_eq!(
            pop.color,
            valence_color(FctValence::Dot),
            "the DOT tick pop is drawn the toxic Dot green valence",
        );
    }

    /// PIN-DISCRIMINATING — the DOT valence is its OWN swatch, distinct from every other FCT
    /// valence (damage / wound / neutral / lethal / suppressed), so a DOT tick reads as a separate
    /// signal and swapping the mapping to a shared valence fails this.
    #[test]
    fn the_dot_valence_differs_from_the_other_valences() {
        let dot = valence_color(FctValence::Dot);
        assert_ne!(
            dot,
            valence_color(FctValence::Damage),
            "the DOT valence must differ from the raw-hit damage RED",
        );
        assert_ne!(
            dot,
            valence_color(FctValence::Wound),
            "the DOT valence must differ from the wound / bleed AMBER",
        );
        assert_ne!(
            dot,
            valence_color(FctValence::Neutral),
            "the DOT valence must differ from the neutral GREY",
        );
        assert_ne!(
            dot,
            valence_color(FctValence::Lethal),
            "the DOT valence must differ from the lethal RED",
        );
        assert_ne!(
            dot,
            valence_color(FctValence::Suppressed),
            "the DOT valence must differ from the cowed suppression blue-grey",
        );
    }
}
