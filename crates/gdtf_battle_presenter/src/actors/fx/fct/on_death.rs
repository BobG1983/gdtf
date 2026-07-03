//! GTW-547 (child GTW-41g): the ON-DEATH floating-combat-text reader — the transient
//! `"BOOM"` blast marker at every cell where an on-death effect FIRED, routed off the sim's
//! [`OnDeathOccurred`](gdtf_battle_sim::OnDeathOccurred) per-death message.
//!
//! # Why this reader exists — the Explode visibility gap
//!
//! Most GTW-547 on-death effects already surface through EXISTING rendering with no presenter
//! work: an [`OnDeathEffect::LeaveField`](gdtf_battle_sim::OnDeathEffect::LeaveField) spawns a
//! GTW-545 field into the sim's [`FieldRegistry`](gdtf_battle_sim::FieldRegistry), which the
//! persistent [`draw_field_overlay`](crate::draw_field_overlay) draws automatically (it polls the
//! registry every frame), plus its per-round [`FieldTicked`](gdtf_battle_sim::FieldTicked)
//! attrition pop. An [`OnDeathEffect::Explode`](gdtf_battle_sim::OnDeathEffect::Explode), by
//! contrast, applies its blast as a DIRECT, deterministic, RNG-free [`Hp`](gdtf_battle_sim::Hp)
//! drain inside the sim's [`resolve_on_death`](gdtf_battle_sim::resolve_on_death) system (chosen to
//! keep the seeded auto-battle stream byte-stable — it takes no shot / injury RNG). That drain
//! emits NO [`ShotFired`](gdtf_battle_sim::ShotFired) and NO
//! [`FieldTicked`](gdtf_battle_sim::FieldTicked), so it rides NEITHER the AoE-shot impact FX NOR an
//! attrition pop — the blast would otherwise be INVISIBLE (a corpse's neighbours silently lose HP).
//!
//! This reader closes that gap with a minimal, presenter-only FLOURISH: one bold blast marker at
//! each [`OnDeathOccurred`](gdtf_battle_sim::OnDeathOccurred) cell — the death / cover-destruction
//! point an on-death effect fanned from — so an explosion (and every terminal death that carries an
//! on-death effect) reads on screen. It rides the LETHAL blood-red valence
//! ([`FctValence::Lethal`](super::palette::FctValence::Lethal), drawn bold), because the message
//! marks a terminal death, not a recurring attrition tick.
//!
//! It MIRRORS the GTW-545 [`FieldTicked`](gdtf_battle_sim::FieldTicked) FCT-pop precedent
//! ([`read_field_fct`](super::read_field_fct)): it resolves the cell straight off the message's
//! [`at`](gdtf_battle_sim::OnDeathOccurred::at) (no `Position` lookup — the message already carries
//! the cell), and stacks simultaneous same-cell markers via an
//! [`FctStackIndex`](super::text::FctStackIndex) counter so two on-death markers on one cell fan
//! out vertically. The classify half is the pure, World-free [`on_death_pop`] so the tag + valence
//! mapping is unit-testable with no [`App`](bevy::prelude::App).
//!
//! Pure VIEW (ADR-0001): it only READS the message, then SPAWNS a presenter pop; it never polls raw
//! sim state and never writes the sim.

use std::collections::HashMap;

use bevy::prelude::*;
use gdtf_battle_sim::OnDeathOccurred;

use super::{
    super::FxTuning,
    palette::{FctValence, valence_color},
    text::{CombatText, FctEmphasis, FctStackIndex, spawn_floating_text},
};

/// The blast-marker string every on-death pop renders — a short, all-caps genre "detonation" tag
/// so an on-death effect (the Explode blast especially) reads at its origin cell.
///
/// Framework plumbing (a literal fed into a [`CombatText`], not a domain quantity — the
/// message carries no per-cell number to render, unlike the DOT / field `"-N"` pops).
const ON_DEATH_MARKER: &str = "BOOM";

/// One ready-to-spawn on-death marker — the [`ON_DEATH_MARKER`] blast string + its lethal
/// blood-red color, before it is anchored at the death cell and given its stack slot.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color)` tuple), mirroring the DOT
/// `DotTickPop` / field `FieldTickPop` shape: [`on_death_pop`] builds it, and [`read_on_death_fct`]
/// anchors it. The [`Color`](bevy::prelude::Color) is framework plumbing (the swatch fed straight
/// to the primitive), the only bare type the no-bare-types rule permits here.
#[derive(Debug, Clone)]
pub(in crate::actors::fx) struct OnDeathPop {
    /// The combat-text string this pop renders — the [`ON_DEATH_MARKER`] blast tag.
    text:  CombatText,
    /// The valence swatch the pop is drawn in — the lethal
    /// [`FctValence::Lethal`](super::palette::FctValence::Lethal) blood-red.
    color: Color,
}

/// Classify an on-death occurrence into the FCT marker it yields — the pure, World-free mapping
/// (GTW-547).
///
/// The pop renders the [`ON_DEATH_MARKER`] blast tag drawn in the lethal
/// [`FctValence::Lethal`](super::palette::FctValence::Lethal) blood-red — a terminal-death signal
/// (the same blood family the DOWN / DEAD shot pop uses), NOT a recurring-attrition valence.
/// Split out of [`read_on_death_fct`] so the tag + valence mapping is unit-testable with no
/// [`App`](bevy::prelude::App). Param-free — the message carries no per-cell number to render, so
/// every on-death marker is the same tag + valence (the cascade / effect variety is a SIM concern).
#[must_use]
pub(in crate::actors::fx) fn on_death_pop() -> OnDeathPop {
    OnDeathPop {
        text:  CombatText::new(ON_DEATH_MARKER.to_owned()),
        color: valence_color(FctValence::Lethal),
    }
}

/// `Update` (`PresenterSystems::Draw`): drain
/// [`OnDeathOccurred`](gdtf_battle_sim::OnDeathOccurred) and spawn one bold blast marker per
/// on-death effect that fanned this frame (GTW-547).
///
/// For each buffered death it:
///
/// 1. Classifies the marker via [`on_death_pop`] — the [`ON_DEATH_MARKER`] blast tag drawn in the
///    lethal [`FctValence::Lethal`](super::palette::FctValence::Lethal) blood-red.
/// 2. Reads the death `(cell, level)` straight off the message's
///    [`at`](gdtf_battle_sim::OnDeathOccurred::at)
///    [`CellLevel`](gdtf_battle_sim::CellLevel), reconstructing the typed
///    [`Cell`](gdtf_battle_sim::Cell) / [`Level`](gdtf_battle_sim::Level) from
///    its `IVec3` (the DOT / field reader idiom — the message already carries the cell, so no
///    `Position` lookup is needed; a cover death carries [`Entity::PLACEHOLDER`] and still has a
///    valid cell).
/// 3. Spawns the marker via [`spawn_floating_text`] at the next per-cell [`FctStackIndex`] (so two
///    on-death markers on one cell — a cascade explosion — fan out vertically), at BOLD weight
///    ([`FctEmphasis::Bold`] — the death / detonation is the heaviest, most-urgent pop), with the
///    hot-reloadable [`FxTuning`] lifetime + rise.
///
/// The marker is the message's ONLY presenter job (the moment the effect fires): a
/// [`LeaveField`](gdtf_battle_sim::OnDeathEffect::LeaveField)'s persistent field is drawn by the
/// [`draw_field_overlay`](crate::draw_field_overlay) reading the sim registry, and an
/// [`Explode`](gdtf_battle_sim::OnDeathEffect::Explode)'s per-victim HP loss is a sim state change
/// — this pop is the transient "an on-death effect fired HERE" flash over both.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the
/// [`MessageReader`](bevy::ecs::message::MessageReader), and [`Res<FxTuning>`] for the
/// hot-reloadable pop lifetime + rise. Its plugin gate adds `resource_exists::<FxTuning>` + the
/// message buffer so the params are always valid (`bevy-traps.md` #1 / #4). It never writes the
/// sim.
pub fn read_on_death_fct(
    mut commands: Commands,
    mut deaths: MessageReader<OnDeathOccurred>,
    tuning: Res<FxTuning>,
) {
    // The per-cell stack counter for THIS frame, keyed by the integer cell + level so two on-death
    // markers on one cell (a cascade explosion) fan one step further down each and distinct cells
    // never share a slot (the DOT / field reader's same-cell stacking precedent).
    let mut stacks: HashMap<(i32, i32, u8), usize> = HashMap::new();

    for message in deaths.read() {
        let pop = on_death_pop();

        // The death cell is carried directly on the message (the sim resolved it at the terminal
        // gate) — the canonical CellLevel::split decompose (GTW-565); no Position lookup, and no
        // fail-closed branch (the message always carries a valid key, cover deaths included).
        let (cell, level) = message.at.split();

        let slot = stacks.entry((cell.x, cell.y, *level)).or_insert(0);
        spawn_floating_text(
            &mut commands,
            pop.text,
            pop.color,
            FctEmphasis::Bold,
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
    use super::{FctValence, ON_DEATH_MARKER, on_death_pop, valence_color};

    /// An on-death occurrence classifies to the blast marker drawn in the lethal blood-red — the
    /// terminal-death detonation tag, in the lethal valence (NOT an attrition green / orange).
    #[test]
    fn an_on_death_classifies_to_a_lethal_blast_marker() {
        let pop = on_death_pop();
        assert_eq!(
            &*pop.text, ON_DEATH_MARKER,
            "the on-death pop renders the blast marker tag",
        );
        assert_eq!(
            pop.color,
            valence_color(FctValence::Lethal),
            "the on-death pop is drawn the lethal blood-red valence (a terminal death)",
        );
    }

    /// PIN-DISCRIMINATING — the on-death marker rides the LETHAL valence, NOT the recurring
    /// DOT / Field attrition valences: an explosion / terminal death is not a per-round tick, so
    /// swapping the mapping to a shared attrition swatch fails this.
    #[test]
    fn the_on_death_marker_is_lethal_not_attrition() {
        let on_death = on_death_pop().color;
        assert_ne!(
            on_death,
            valence_color(FctValence::Dot),
            "the on-death marker must differ from the DOT toxic green (not an attrition tick)",
        );
        assert_ne!(
            on_death,
            valence_color(FctValence::Field),
            "the on-death marker must differ from the field hazard orange (not an attrition tick)",
        );
    }
}
