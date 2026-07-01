//! GTW-526 (C8): the SUPPRESSION floating-combat-text reader — the transient `"SUPPRESSED"`
//! pop for a ganger freshly pinned down by incoming fire, routed off the GTW-526
//! [`SuppressionApplied`](gdtf_battle_sim::SuppressionApplied) boundary message.
//!
//! When the sim's suppression producer marks an opposing ganger
//! [`Suppressed`](gdtf_battle_sim::Suppressed) within a shot's
//! radius (GTW-526 C2) it emits one [`SuppressionApplied`](gdtf_battle_sim::SuppressionApplied)
//! carrying the pinned ganger's `(cell, level)`. This module routes that to a rise-and-fade FCT
//! pop reading `"SUPPRESSED"` over the cell, drawn in the cowed
//! [`FctValence::Suppressed`](super::palette::FctValence::Suppressed) blue-grey — the same
//! colour-drained family the suppressed sprite tint uses (`stance_aiming_tint`), so the transient
//! pop and the persistent sprite desaturation read as ONE signal. The pop is a MOMENT signal (the
//! instant the pin lands); the persistent state is the sprite tint, NOT this one-shot pop.
//!
//! It MIRRORS the slice-4 consequence reader
//! ([`read_consequence_fct`](super::read_consequence_fct)) and the injury reader
//! ([`read_injury_fct`](super::read_injury_fct)): it resolves the pinned cell (here straight off
//! the message's [`at`](gdtf_battle_sim::SuppressionApplied::at) — no `Position` lookup, since
//! suppression is anchored at a cell, not an entity), and stacks simultaneous same-cell pops via
//! an [`FctStackIndex`](super::text::FctStackIndex) counter so two suppressions on one cell this
//! tick fan out vertically instead of overlapping. The classify half is the pure, World-free
//! [`suppression_pop`] so the valence mapping is unit-testable with no [`App`](bevy::prelude::App).
//!
//! Pure VIEW (ADR-0001): it only READS the message, then SPAWNS a presenter pop; it never polls
//! raw sim state and never writes the sim.

use std::collections::HashMap;

use bevy::prelude::*;
use gdtf_battle_sim::{Cell, Level, SuppressionApplied};

use super::{
    super::FxTuning,
    palette::{FctValence, valence_color},
    text::{CombatText, FctEmphasis, FctStackIndex, spawn_floating_text},
};

/// One ready-to-spawn suppression pop — the `"SUPPRESSED"` string + its cowed valence color,
/// before it is anchored at the pinned cell and given its stack slot.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color)` tuple), mirroring the slice-3
/// `ClassifiedPop` / slice-4 `AuxPop` / injury `InjuryPop` shape: [`suppression_pop`] builds it,
/// and [`read_suppression_fct`] anchors it. The [`Color`](bevy::prelude::Color) is framework
/// plumbing (the swatch fed straight to the primitive), the only bare type the no-bare-types
/// rule permits here.
#[derive(Debug, Clone)]
pub(in crate::actors::fx) struct SuppressionPop {
    /// The combat-text string this pop renders — the fixed `"SUPPRESSED"` tag.
    text:  CombatText,
    /// The valence swatch the pop is drawn in — the cowed
    /// [`FctValence::Suppressed`](super::palette::FctValence::Suppressed) blue-grey.
    color: Color,
}

/// Classify a suppression event into the FCT pop it yields — the pure, World-free mapping
/// (GTW-526 C8).
///
/// The pop renders the fixed `"SUPPRESSED"` tag drawn in the cowed
/// [`FctValence::Suppressed`](super::palette::FctValence::Suppressed) blue-grey (the
/// colour-drained morale family, distinct from the damage RED / wound AMBER / neutral GREY /
/// lethal RED). Split out of [`read_suppression_fct`] so the valence mapping is unit-testable with
/// no [`App`](bevy::prelude::App) (the FCT-routing classify test).
#[must_use]
pub(in crate::actors::fx) fn suppression_pop() -> SuppressionPop {
    SuppressionPop {
        text:  CombatText::new("SUPPRESSED"),
        color: valence_color(FctValence::Suppressed),
    }
}

/// `Update` (`PresenterSystems::Draw`): drain
/// [`SuppressionApplied`](gdtf_battle_sim::SuppressionApplied) and spawn one `"SUPPRESSED"`
/// floating-combat-text pop per freshly-pinned ganger (GTW-526 C8).
///
/// For each drained message it:
///
/// 1. Classifies the pop via [`suppression_pop`] — the fixed `"SUPPRESSED"` tag drawn in the
///    cowed [`FctValence::Suppressed`](super::palette::FctValence::Suppressed) blue-grey.
/// 2. Reads the pinned `(cell, level)` straight off the message's
///    [`at`](gdtf_battle_sim::SuppressionApplied::at) [`CellLevel`](gdtf_battle_sim::CellLevel),
///    reconstructing the typed [`Cell`] / [`Level`] from its `IVec3` (the readers.rs
///    `cell_and_level` idiom — the message already carries the cell, so no `Position` lookup is
///    needed).
/// 3. Spawns the pop via [`spawn_floating_text`] at the next per-cell [`FctStackIndex`] (so two
///    suppressions on one cell this tick fan out vertically), at body weight
///    ([`FctEmphasis::Normal`] — bold is reserved for the lethal DOWN / DEAD tag), with the
///    hot-reloadable [`FxTuning`] lifetime + rise (GTW-327).
///
/// The pop is the message's ONLY presenter job (the MOMENT the pin lands): the PERSISTENT
/// suppressed look is the desaturated sprite tint driven by the
/// [`Suppressed`](gdtf_battle_sim::Suppressed) component in `reframe_ganger_sprites`, NOT this
/// transient flash.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the
/// [`MessageReader`](bevy::ecs::message::MessageReader), and [`Res<FxTuning>`] for the
/// hot-reloadable pop lifetime + rise. Its plugin gate adds `resource_exists::<FxTuning>` + the
/// message buffer so the params are always valid (`bevy-traps.md` #1 / #4). It never writes the
/// sim.
pub fn read_suppression_fct(
    mut commands: Commands,
    mut applied: MessageReader<SuppressionApplied>,
    tuning: Res<FxTuning>,
) {
    // The per-cell stack counter for THIS drain, keyed by the integer cell + level so two
    // suppressions on one cell fan one step further down each and distinct cells never share a
    // slot (the consequence / injury reader's same-cell stacking precedent).
    let mut stacks: HashMap<(i32, i32, u8), usize> = HashMap::new();

    for message in applied.read() {
        let pop = suppression_pop();

        // The pinned cell is carried directly on the message (suppression is anchored at a cell,
        // not an entity), so reconstruct the typed Cell / Level from its IVec3 — no Position
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
    use super::{FctValence, suppression_pop, valence_color};

    /// A suppression event classifies to the cowed `"SUPPRESSED"` blue-grey pop — the fixed tag
    /// drawn in the [`FctValence::Suppressed`] valence (C8: the moment-pop signal).
    #[test]
    fn a_suppression_classifies_to_a_cowed_suppressed_tag() {
        let pop = suppression_pop();
        assert_eq!(
            &*pop.text, "SUPPRESSED",
            "the suppression consequence pops the \"SUPPRESSED\" tag",
        );
        assert_eq!(
            pop.color,
            valence_color(FctValence::Suppressed),
            "the suppression pop is drawn the cowed Suppressed blue-grey valence",
        );
    }

    /// PIN-DISCRIMINATING — the suppression valence is its OWN swatch, distinct from every other
    /// FCT valence (damage / wound / neutral / lethal), so a suppressed ganger's pop reads as a
    /// separate signal and swapping the mapping to a shared valence fails this.
    #[test]
    fn the_suppression_valence_differs_from_the_other_valences() {
        let suppressed = valence_color(FctValence::Suppressed);
        assert_ne!(
            suppressed,
            valence_color(FctValence::Damage),
            "the suppression valence must differ from the damage RED",
        );
        assert_ne!(
            suppressed,
            valence_color(FctValence::Wound),
            "the suppression valence must differ from the wound AMBER",
        );
        assert_ne!(
            suppressed,
            valence_color(FctValence::Neutral),
            "the suppression valence must differ from the neutral GREY",
        );
        assert_ne!(
            suppressed,
            valence_color(FctValence::Lethal),
            "the suppression valence must differ from the lethal RED",
        );
    }
}
