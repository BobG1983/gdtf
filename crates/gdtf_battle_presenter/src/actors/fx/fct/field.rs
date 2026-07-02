//! GTW-545 (child GTW-41f): the AREA-DAMAGE-FIELD floating-combat-text reader — the transient
//! `"-N"` attrition pop for a ganger a persistent damage ZONE (a toxic-waste pool, an
//! electrified floor, a patch of burning ground) drained this round, routed off the sim's
//! [`FieldTicked`](gdtf_battle_sim::FieldTicked) per-round message.
//!
//! Each round the sim's [`tick_fields`](gdtf_battle_sim::tick_fields) clock drains a flat
//! [`amount`](gdtf_battle_sim::FieldTicked::amount) of HP off the ganger standing on a live field
//! and emits one [`FieldTicked`](gdtf_battle_sim::FieldTicked) carrying the occupant, its
//! `(cell, level)`, and the HP drained. This module routes that to a rise-and-fade FCT pop reading
//! `"-N"` (the drained HP) over the field cell, drawn in the hazard
//! [`FctValence::Field`](super::palette::FctValence::Field) orange — its OWN environmental
//! attrition valence, distinct from the raw-hit damage RED, the wound AMBER, the bleed status tag,
//! the neutral GREY, the lethal RED, the cowed suppression blue-grey, and the DOT toxic green — so
//! a recurring field tick reads as its own signal rather than being mistaken for a fresh weapon
//! hit or a carried DOT affliction.
//!
//! It MIRRORS the GTW-544 [`DotTicked`](gdtf_battle_sim::DotTicked) FCT-pop precedent
//! ([`read_dot_fct`](super::read_dot_fct)): it resolves the drained cell straight off the message's
//! [`at`](gdtf_battle_sim::FieldTicked::at) (no `Position` lookup — the message already carries the
//! cell), and stacks simultaneous same-cell pops via an
//! [`FctStackIndex`](super::text::FctStackIndex) counter so two field ticks on one cell this round
//! fan out vertically instead of overlapping. The classify half is the pure, World-free
//! [`field_tick_pop`] so the tag + valence mapping is unit-testable with no [`App`](bevy::prelude::App).
//!
//! Pure VIEW (ADR-0001): it only READS the message, then SPAWNS a presenter pop; it never polls
//! raw sim state and never writes the sim.

use std::collections::HashMap;

use bevy::prelude::*;
use gdtf_battle_sim::{Cell, FieldDamage, FieldTicked, Level};

use super::{
    super::FxTuning,
    palette::{FctValence, valence_color},
    text::{CombatText, FctEmphasis, FctStackIndex, spawn_floating_text},
};

/// One ready-to-spawn field-tick pop — the `"-N"` drained-HP string + its hazard field color,
/// before it is anchored at the field cell and given its stack slot.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color)` tuple), mirroring the DOT
/// `DotTickPop` shape: [`field_tick_pop`] builds it, and [`read_field_fct`] anchors it. The
/// [`Color`](bevy::prelude::Color) is framework plumbing (the swatch fed straight to the
/// primitive), the only bare type the no-bare-types rule permits here.
#[derive(Debug, Clone)]
pub(in crate::actors::fx) struct FieldTickPop {
    /// The combat-text string this pop renders — the drained HP as a `"-N"` number.
    text:  CombatText,
    /// The valence swatch the pop is drawn in — the hazard
    /// [`FctValence::Field`](super::palette::FctValence::Field) orange.
    color: Color,
}

/// Classify a field tick of `amount` HP into the FCT pop it yields — the pure, World-free mapping
/// (GTW-545).
///
/// The pop renders the drained HP as a `"-N"` number (the same `-{hp}` shape the per-shot damage
/// pop and the DOT tick pop use, so a field tick reads as a familiar HP-loss number) drawn in the
/// hazard [`FctValence::Field`](super::palette::FctValence::Field) orange — its own recurring
/// environmental-attrition valence, distinct from every other FCT valence. Split out of
/// [`read_field_fct`] so the tag + valence mapping is unit-testable with no [`App`](bevy::prelude::App).
#[must_use]
pub(in crate::actors::fx) fn field_tick_pop(amount: FieldDamage) -> FieldTickPop {
    FieldTickPop {
        text:  CombatText::new(format!("-{}", *amount)),
        color: valence_color(FctValence::Field),
    }
}

/// `Update` (`PresenterSystems::Draw`): drain [`FieldTicked`](gdtf_battle_sim::FieldTicked) and
/// spawn one `"-N"` floating-combat-text pop per drained ganger's per-round field drain (GTW-545).
///
/// For each drained message it:
///
/// 1. Classifies the pop via [`field_tick_pop`] — the drained HP as a `"-N"` number drawn in the
///    hazard [`FctValence::Field`](super::palette::FctValence::Field) orange.
/// 2. Reads the drained `(cell, level)` straight off the message's
///    [`at`](gdtf_battle_sim::FieldTicked::at) [`CellLevel`](gdtf_battle_sim::CellLevel),
///    reconstructing the typed [`Cell`] / [`Level`] from its `IVec3` (the readers.rs
///    `cell_and_level` idiom — the message already carries the cell, so no `Position` lookup is
///    needed).
/// 3. Spawns the pop via [`spawn_floating_text`] at the next per-cell [`FctStackIndex`] (so two
///    field ticks on one cell this round fan out vertically), at body weight
///    ([`FctEmphasis::Normal`] — bold is reserved for the lethal DOWN / DEAD tag), with the
///    hot-reloadable [`FxTuning`] lifetime + rise (GTW-327).
///
/// The pop is the message's ONLY presenter job (the moment the tick lands): the persistent field
/// ZONE lives on the sim's [`FieldRegistry`](gdtf_battle_sim::FieldRegistry), drawn as the
/// persistent per-cell overlay ([`draw_field_overlay`](crate::draw_field_overlay)), NOT this
/// transient flash.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the
/// [`MessageReader`](bevy::ecs::message::MessageReader), and [`Res<FxTuning>`] for the
/// hot-reloadable pop lifetime + rise. Its plugin gate adds `resource_exists::<FxTuning>` + the
/// message buffer so the params are always valid (`bevy-traps.md` #1 / #4). It never writes the
/// sim.
pub fn read_field_fct(
    mut commands: Commands,
    mut ticked: MessageReader<FieldTicked>,
    tuning: Res<FxTuning>,
) {
    // The per-cell stack counter for THIS drain, keyed by the integer cell + level so two field
    // ticks on one cell fan one step further down each and distinct cells never share a slot
    // (the DOT reader's same-cell stacking precedent).
    let mut stacks: HashMap<(i32, i32, u8), usize> = HashMap::new();

    for message in ticked.read() {
        let pop = field_tick_pop(message.amount);

        // The drained cell is carried directly on the message (the sim resolved the occupant's
        // cell at tick time), so reconstruct the typed Cell / Level from its IVec3 — no Position
        // lookup, and no fail-closed branch (the message always carries a valid key).
        let at = message.at;
        let cell = Cell::new(at.x, at.y);
        let storey = u8::try_from(at.z).unwrap_or(0);
        let level = Level::new(storey);

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
    use gdtf_battle_sim::FieldDamage;

    use super::{FctValence, field_tick_pop, valence_color};

    /// A field tick classifies to a `"-N"` drained-HP pop drawn in the hazard Field orange — the
    /// amount is rendered as a familiar `-{hp}` number, in the field attrition valence.
    #[test]
    fn a_field_tick_classifies_to_a_hazard_minus_amount_tag() {
        let pop = field_tick_pop(FieldDamage::new(3));
        assert_eq!(
            &*pop.text, "-3",
            "the field tick pops the drained HP as a \"-N\" number",
        );
        assert_eq!(
            pop.color,
            valence_color(FctValence::Field),
            "the field tick pop is drawn the hazard Field orange valence",
        );
    }

    /// PIN-DISCRIMINATING — the Field valence is its OWN swatch, distinct from every other FCT
    /// valence (damage / wound / neutral / lethal / suppressed / DOT), so a field tick reads as a
    /// separate signal and swapping the mapping to a shared valence fails this.
    #[test]
    fn the_field_valence_differs_from_the_other_valences() {
        let field = valence_color(FctValence::Field);
        assert_ne!(
            field,
            valence_color(FctValence::Damage),
            "the Field valence must differ from the raw-hit damage RED",
        );
        assert_ne!(
            field,
            valence_color(FctValence::Wound),
            "the Field valence must differ from the wound / bleed AMBER",
        );
        assert_ne!(
            field,
            valence_color(FctValence::Neutral),
            "the Field valence must differ from the neutral GREY",
        );
        assert_ne!(
            field,
            valence_color(FctValence::Lethal),
            "the Field valence must differ from the lethal RED",
        );
        assert_ne!(
            field,
            valence_color(FctValence::Suppressed),
            "the Field valence must differ from the cowed suppression blue-grey",
        );
        assert_ne!(
            field,
            valence_color(FctValence::Dot),
            "the Field valence must differ from the DOT toxic green",
        );
    }
}
