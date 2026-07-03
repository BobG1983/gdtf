//! GTW-439 (slice C1): the INJURY floating-combat-text reader — the transient flash for a
//! freshly-inflicted named injury, routed off the GTW-438
//! [`InjuryInflicted`](gdtf_battle_sim::InjuryInflicted) boundary message.
//!
//! When the injury trigger (GTW-438) rolls a named injury on a wounded ganger it emits one
//! [`InjuryInflicted`](gdtf_battle_sim::InjuryInflicted) carrying the wounded `target`, the
//! rolled [`severity`](gdtf_battle_sim::InjuryInflicted::severity), and the three routed
//! texts. This module routes its [`popup_text`](gdtf_battle_sim::InjuryInflicted::popup_text)
//! to a rise-and-fade FCT pop over the wounded ganger's cell, drawn in a VALENCE BY SEVERITY:
//! a worse rolled tier reads HOTTER in the wound-family amber ramp the FCT palette already
//! owns ([`severity_color`](super::palette::severity_color)) — `Minor` the light amber base,
//! `Critical` the hot orange-red one tier shy of the lethal blood red. The transient flash is
//! the message's ONLY presenter job: the PERSISTENT per-ganger injury list is driven by the
//! durable [`InflictedInjuries`](gdtf_battle_sim::InflictedInjuries) ledger in the inspect
//! panel (GTW-439 slice C3), NOT by this one-shot pop.
//!
//! It MIRRORS the slice-4 consequence reader ([`read_consequence_fct`](super::read_consequence_fct)):
//! it resolves the wounded ganger's cell from its [`Position`](gdtf_battle_sim::Position),
//! fail-closed (a `Position`-less ganger spawns no pop), and stacks simultaneous same-cell
//! pops via an [`FctStackIndex`](super::text::FctStackIndex) counter so two injuries on one
//! cell this tick fan out vertically instead of overlapping. The valence mapping is split into
//! the pure, World-free [`injury_pop`] classifier so it is unit-testable with no
//! [`App`](bevy::prelude::App) (the FCT-routing classify test, C4).
//!
//! Pure VIEW (ADR-0001): it only READS the message + the ganger's cell, then SPAWNS a
//! presenter pop; it never polls raw sim state and never writes the sim.

use std::collections::HashMap;

use bevy::prelude::*;
use gdtf_battle_sim::{InjuryInflicted, Position, Severity};

use super::{
    super::FxTuning,
    palette::severity_color,
    text::{CombatText, FctEmphasis, FctStackIndex, spawn_floating_text},
};

/// One ready-to-spawn injury pop — the popup string + its severity-scaled valence color,
/// before it is anchored at the wounded ganger's cell and given its stack slot.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color)` tuple), mirroring the
/// slice-3 `ClassifiedPop` / slice-4 `AuxPop` shape: [`injury_pop`] builds it from the
/// message, and [`read_injury_fct`] anchors it. The [`Color`](bevy::prelude::Color) is
/// framework plumbing (the swatch fed straight to the primitive), the only bare type the
/// no-bare-types rule permits here.
#[derive(Debug, Clone)]
pub(in crate::actors::fx) struct InjuryPop {
    /// The combat-text string this pop renders — the injury's authored
    /// [`popup_text`](gdtf_battle_sim::InjuryInflicted::popup_text) (e.g. `"LOST EYE"`).
    /// Read directly within this module (the sibling `AuxPop` precedent — a small in-module
    /// value object whose fields the reader + the classify test touch by name).
    text:  CombatText,
    /// The valence swatch the pop is drawn in — the
    /// [`severity_color`](super::palette::severity_color) amber ramp scaled by the rolled
    /// tier (a worse injury reads hotter).
    color: Color,
}

/// Classify a freshly-inflicted injury's `popup_text` + rolled `severity` into the FCT pop
/// it yields — the pure, World-free valence-by-severity mapping (GTW-439 C1).
///
/// The pop renders the authored `popup_text` verbatim, drawn in the
/// [`severity_color`](super::palette::severity_color) wound-family ramp scaled by the rolled
/// tier: a [`Minor`](Severity::Minor) injury reads the light amber base, a
/// [`Critical`](Severity::Critical) injury the hot orange-red one tier shy of the lethal
/// red — so a WORSE rolled tier reads a HOTTER pop, consistent with the existing FCT wound
/// palette (the shot-classifier's wound tags use the SAME ramp). Only `Minor` / `Major` /
/// `Critical` are tabled injuries (GTW-405), but the ramp is total over every
/// [`Severity`], so a defensive `None` / `Fatal` still resolves to a sane swatch.
///
/// Split out of [`read_injury_fct`] so the valence mapping is unit-testable with no
/// [`App`](bevy::prelude::App) (C4 — feed a `(text, severity)` and assert the exact pop).
#[must_use]
pub(in crate::actors::fx) fn injury_pop(popup_text: &str, severity: Severity) -> InjuryPop {
    InjuryPop {
        text:  CombatText::new(popup_text),
        color: severity_color(severity),
    }
}

/// `Update` (`PresenterSystems::Draw`): drain [`InjuryInflicted`](gdtf_battle_sim::InjuryInflicted)
/// and spawn one severity-valenced floating-combat-text pop per inflicted injury (GTW-439 C1).
///
/// For each drained message it:
///
/// 1. Classifies the pop via [`injury_pop`] — the message's
///    [`popup_text`](gdtf_battle_sim::InjuryInflicted::popup_text) drawn in the
///    [`severity_color`](super::palette::severity_color) ramp scaled by the rolled
///    [`severity`](gdtf_battle_sim::InjuryInflicted::severity).
/// 2. Looks up the wounded [`target`](gdtf_battle_sim::InjuryInflicted::target) ganger's cell
///    via `Query<&Position>.get(target)`, decomposed via the canonical
///    [`CellLevel::split`](gdtf_battle_sim::CellLevel::split) (GTW-565). A ganger with no
///    [`Position`] is skipped FAIL-CLOSED (no panic, no pop).
/// 3. Spawns the pop via [`spawn_floating_text`] at the next per-cell [`FctStackIndex`] (so
///    two injuries inflicted on one cell this tick fan out vertically), at body weight
///    ([`FctEmphasis::Normal`] — bold is reserved for the lethal DOWN / DEAD tag), with the
///    hot-reloadable [`FxTuning`] lifetime + rise (GTW-327).
///
/// The pop is the message's ONLY presenter job: the durable per-ganger injury list is driven
/// by the [`InflictedInjuries`](gdtf_battle_sim::InflictedInjuries) ledger in the inspect
/// panel, NOT by this transient flash.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the read-only `Query<&Position>` for the
/// ganger anchor, the [`MessageReader`](bevy::ecs::message::MessageReader), and
/// [`Res<FxTuning>`] for the hot-reloadable pop lifetime + rise. Its plugin gate adds
/// `resource_exists::<FxTuning>` + the message buffer so the params are always valid
/// (`bevy-traps.md` #1 / #4). It never writes the sim.
pub fn read_injury_fct(
    mut commands: Commands,
    positions: Query<&Position>,
    mut injuries: MessageReader<InjuryInflicted>,
    tuning: Res<FxTuning>,
) {
    // The per-cell stack counter for THIS drain, keyed by the integer cell + level so two
    // injuries on one cell fan one step further down each and distinct cells never share a
    // slot (the consequence reader's same-cell stacking precedent).
    let mut stacks: HashMap<(i32, i32, u8), usize> = HashMap::new();

    for message in injuries.read() {
        let pop = injury_pop(&message.popup_text, message.severity);

        // Fail-closed: a ganger with no Position spawns no pop (its cell is unknown), no panic.
        let Ok(position) = positions.get(message.target) else {
            continue;
        };
        // The canonical CellLevel::split decompose through Position's deref (GTW-565).
        let (cell, level) = position.split();

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
    use gdtf_battle_sim::Severity;

    use super::{injury_pop, severity_color};

    /// The injury FCT pop renders the authored `popup_text` verbatim and draws it in the
    /// `severity_color` ramp scaled by the rolled tier (C1: valence BY SEVERITY).
    #[test]
    fn an_injury_pop_renders_the_popup_text_in_the_severity_color() {
        let pop = injury_pop("LOST EYE", Severity::Critical);
        assert_eq!(
            &*pop.text, "LOST EYE",
            "the injury pop renders the authored popup_text verbatim",
        );
        assert_eq!(
            pop.color,
            severity_color(Severity::Critical),
            "the injury pop is drawn the severity-scaled wound valence",
        );
    }

    /// PIN-DISCRIMINATING — the valence tracks the rolled tier: a WORSE injury reads a
    /// DIFFERENT (hotter) swatch than a milder one, so the mapping is genuinely
    /// severity-scaled, not a flat constant color. A `Minor` and a `Critical` pop must
    /// differ, and each must match its OWN tier's `severity_color` (so swapping the mapping
    /// to a flat valence — or to the wrong tier — fails this).
    #[test]
    fn the_injury_valence_scales_with_severity() {
        let minor = injury_pop("BRUISE", Severity::Minor);
        let major = injury_pop("GASH", Severity::Major);
        let critical = injury_pop("LOST EYE", Severity::Critical);

        assert_eq!(
            minor.color,
            severity_color(Severity::Minor),
            "a Minor injury reads the Minor amber",
        );
        assert_eq!(
            major.color,
            severity_color(Severity::Major),
            "a Major injury reads the Major amber",
        );
        assert_eq!(
            critical.color,
            severity_color(Severity::Critical),
            "a Critical injury reads the Critical amber",
        );
        // A worse tier must read a DIFFERENT swatch (the ramp climbs) — a flat valence fails.
        assert_ne!(
            minor.color, critical.color,
            "a Critical injury must read a hotter swatch than a Minor one (valence by severity)",
        );
    }
}
