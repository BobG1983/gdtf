//! GTW-302 (slice 4): the AUXILIARY-SIGNAL floating-combat-text reader — the Phase-1 pops
//! that are NOT derivable from [`ShotFired`] alone but ride the dedicated CONSEQUENCE
//! messages the sim already emits.
//!
//! Slice 3 ([`reader`](super::reader)) covered every event a fired round's
//! [`HitReport`](gdtf_battle_sim::HitReport) carries. This slice covers the remaining
//! Phase-1 pops that arrive on their own messages, one tick later in the sim's resolution:
//!
//! - **Bleeding started** — a [`Bleeding`](gdtf_battle_sim::Bleeding) `{ ganger }` (the §9
//!   bleed-out clock drained a Downed ganger this round) pops a `"Bleeding"` tag in the
//!   AMBER wound/status valence ([`FctValence::Wound`](super::palette::FctValence::Wound))
//!   over the bleeding ganger's cell. The pop sits ALONGSIDE the existing
//!   [`read_bleeding`](super::super::read_bleeding) blood flash — the flash is the splash,
//!   this is the labelled tag.
//! - **Armor broke** — an [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) `{ ganger, part }`
//!   (a worn piece crossed from protecting to useless) pops an `"Armor Broken"` tag in the
//!   damage RED ([`FctValence::Damage`](super::palette::FctValence::Damage)) over the
//!   ganger's cell — the destructive event reads heavier than ordinary wear (the contract's
//!   "AMBER/RED" for the destroy crossing). It rides ALONGSIDE the existing
//!   [`read_armor_broken`](super::super::read_armor_broken) spark flash.
//!
//! DEFERRED (not built — no backing sim signal, FLAGGED in the slice-4 handoff):
//!
//! - **`"Armor -N"`** — the [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) message carries NO
//!   integrity-delta amount (verified `armor_wear/wear.rs:31-36` — only `{ ganger, part }`,
//!   the broken CROSSING, not a per-hit wear number), so the numeric `"-N"` variant cannot be
//!   sourced. Only the `"Armor Broken"` crossing tag is built. Surfacing `"Armor -N"` needs a
//!   new sim field on [`ArmorBroken`] (or an integrity-delta signal).
//! - **Reload `"Reloaded"` / `"Empty"` / `"No TU"`** — there is NO reload-RESULT message in
//!   the sim (verified `acts/reload.rs:48-79` — `dispatch_reload` silently mutates `Magazine`
//!   / `Tu` and emits nothing; only the inbound `ReloadRequested` exists). The presenter must
//!   NOT invent one or poll raw sim state (ADR-0001), so reload pops are DEFERRED pending a
//!   new sim reload-result message.
//!
//! Pure VIEW (ADR-0001): it only READS the consequence messages + looks up the ganger's
//! cell, then SPAWNS presenter pops; it never polls raw sim state and never writes the sim.

use bevy::prelude::*;
use gdtf_battle_sim::{ArmorBroken, Bleeding, Position};

use super::{
    super::FxTuning,
    palette::{FctValence, valence_color},
    text::{CombatText, FctEmphasis, FctStackIndex, spawn_floating_text},
};

/// `Update` (`PresenterSystems::Draw`): drain the consequence messages and spawn the
/// auxiliary floating-combat-text pops (bleeding-started, armor-broken).
///
/// It drains BOTH [`MessageReader<Bleeding>`](gdtf_battle_sim::Bleeding) and
/// [`MessageReader<ArmorBroken>`](gdtf_battle_sim::ArmorBroken) — the SAME buffers the
/// [`read_bleeding`](super::super::read_bleeding) / [`read_armor_broken`](super::super::read_armor_broken)
/// blood/spark flash readers drain (a buffered Bevy message survives the frame, so the flash
/// reader and this pop reader see every message independently). For each message it:
///
/// 1. Looks up the ganger's cell via `Query<&Position>.get(msg.ganger)`, decomposed via the
///    canonical [`CellLevel::split`](gdtf_battle_sim::CellLevel::split) (GTW-565). A ganger
///    with no [`Position`] is skipped FAIL-CLOSED (no panic, no pop).
/// 2. Spawns one pop via [`spawn_floating_text`]: a `Bleeding` pops a `"Bleeding"` tag in the
///    AMBER wound/status valence; an `ArmorBroken` pops an `"Armor Broken"` tag in the damage
///    RED (the destroy crossing reads heavier than wear).
/// 3. Assigns each pop the next per-cell [`FctStackIndex`] from a stack counter that persists
///    across BOTH drains, so a bleed + an armor-break on one cell this tick fan out
///    vertically instead of overlapping (matching the slice-3 same-cell stacking).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the read-only `Query<&Position>` for the
/// ganger anchor (the pop sits on the ganger's sim cell, not its rendered stance-height
/// sprite — so the cheaper [`Position`] lookup suffices), the two
/// [`MessageReader`](bevy::ecs::message::MessageReader)s, and [`Res<FxTuning>`] for the
/// hot-reloadable pop lifetime + rise (GTW-327) each spawned pop is given. Its plugin gate adds
/// `resource_exists::<FxTuning>` so the resource is always present when this runs. It never
/// writes the sim.
pub fn read_consequence_fct(
    mut commands: Commands,
    positions: Query<&Position>,
    mut bleeds: MessageReader<Bleeding>,
    mut broken: MessageReader<ArmorBroken>,
    tuning: Res<FxTuning>,
) {
    // The per-cell stack counter for THIS drain, shared across both message kinds so two
    // simultaneous pops on one cell (e.g. a bleed + an armor-break) fan one step further down
    // each. Keyed by the integer cell + level so distinct cells never share a slot.
    let mut stacks: std::collections::HashMap<(i32, i32, u8), usize> =
        std::collections::HashMap::new();

    for msg in bleeds.read() {
        let pop = bleeding_pop();
        spawn_aux_pop(
            &mut commands,
            &positions,
            &mut stacks,
            msg.ganger,
            pop,
            &tuning,
        );
    }

    for msg in broken.read() {
        let pop = armor_broken_pop();
        spawn_aux_pop(
            &mut commands,
            &positions,
            &mut stacks,
            msg.ganger,
            pop,
            &tuning,
        );
    }
}

/// One ready-to-spawn auxiliary pop — the classified string + its valence color, before it is
/// anchored at the ganger's cell and given its stack slot.
///
/// A NAMED grouping struct (not a bare `(CombatText, Color)` tuple), mirroring slice 3's
/// `ClassifiedPop`: the [`bleeding_pop`] / [`armor_broken_pop`] classifiers build it, and
/// [`spawn_aux_pop`] anchors it. The [`Color`](bevy::prelude::Color) is framework plumbing
/// (the swatch fed straight to the primitive), the only bare type the no-bare-types rule
/// permits here.
struct AuxPop {
    /// The combat-text string this pop renders (`"Bleeding"` / `"Armor Broken"`).
    text:  CombatText,
    /// The valence swatch the pop is drawn in (the AMBER wound/status or damage RED family
    /// the consequence maps to).
    color: Color,
}

/// The AMBER `"Bleeding"` status pop a [`Bleeding`](gdtf_battle_sim::Bleeding) message yields.
///
/// A wound/status valence ([`FctValence::Wound`]) — the bleed-out clock drained the ganger,
/// a status the player should clock, drawn in the flat AMBER (the message carries no severity
/// to ramp by, so the flat wound swatch — not [`severity_color`](super::palette::severity_color) — fits).
fn bleeding_pop() -> AuxPop {
    AuxPop {
        text:  CombatText::new("Bleeding"),
        color: valence_color(FctValence::Wound),
    }
}

/// The RED `"Armor Broken"` pop an [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) message yields.
///
/// The destroy CROSSING reads heavier than ordinary wear, so it pops in the damage RED
/// ([`FctValence::Damage`]) — the contract's "AMBER/RED" for the broken event, drawn the
/// redder of the two. The numeric `"Armor -N"` variant is DEFERRED: [`ArmorBroken`] carries no
/// integrity-delta amount (only the `{ ganger, part }` crossing — see the module docs).
fn armor_broken_pop() -> AuxPop {
    AuxPop {
        text:  CombatText::new("Armor Broken"),
        color: valence_color(FctValence::Damage),
    }
}

/// Spawn one auxiliary pop over `ganger`'s cell with the next per-cell stack slot, or do
/// nothing (fail-closed) if the ganger has no [`Position`].
///
/// Shared by both consequence drains: it resolves the ganger's `(cell, level)` anchor from
/// the `Query<&Position>` (reconstructing the typed [`Cell`] / [`Level`] from the position's
/// `IVec3`), pulls the next [`FctStackIndex`] for that cell out of the shared `stacks`
/// counter, and spawns the pop with the hot-reloadable [`FxTuning`] lifetime + rise (GTW-327).
/// A `Position`-less ganger is skipped — no panic, no pop.
fn spawn_aux_pop(
    commands: &mut Commands,
    positions: &Query<&Position>,
    stacks: &mut std::collections::HashMap<(i32, i32, u8), usize>,
    ganger: Entity,
    pop: AuxPop,
    tuning: &FxTuning,
) {
    // Fail-closed: a ganger with no Position spawns no pop (its cell is unknown), no panic.
    let Ok(pos) = positions.get(ganger) else {
        return;
    };
    // The canonical CellLevel::split decompose through Position's deref (GTW-565).
    let (cell, level) = pos.split();

    let slot = stacks.entry((cell.x, cell.y, *level)).or_insert(0);
    // Auxiliary status tags (bleeding, armor-broken) are body-weight — bold is reserved for
    // the lethal DOWN / DEAD tag (slice 3).
    spawn_floating_text(
        commands,
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

#[cfg(test)]
mod test {
    use super::{FctValence, armor_broken_pop, bleeding_pop, valence_color};

    /// A `Bleeding` consequence classifies to the AMBER `"Bleeding"` status pop (the flat
    /// wound/status swatch — no severity to ramp by).
    #[test]
    fn a_bleeding_classifies_to_an_amber_bleeding_tag() {
        let pop = bleeding_pop();
        assert_eq!(
            &*pop.text, "Bleeding",
            "the bleeding consequence pops the \"Bleeding\" tag",
        );
        assert_eq!(
            pop.color,
            valence_color(FctValence::Wound),
            "the bleeding pop is drawn the AMBER wound/status valence",
        );
    }

    /// An `ArmorBroken` consequence classifies to the RED `"Armor Broken"` pop — the destroy
    /// crossing reads heavier than ordinary wear, distinct from the AMBER wound family. (The
    /// numeric `"Armor -N"` variant is DEFERRED: the message carries no integrity-delta amount.)
    #[test]
    fn an_armor_broken_classifies_to_a_red_armor_broken_tag() {
        let pop = armor_broken_pop();
        assert_eq!(
            &*pop.text, "Armor Broken",
            "the armor-broken consequence pops the \"Armor Broken\" tag",
        );
        assert_eq!(
            pop.color,
            valence_color(FctValence::Damage),
            "the armor-broken pop is drawn the damage RED (the destroy crossing reads heavier)",
        );
        assert_ne!(
            pop.color,
            valence_color(FctValence::Wound),
            "the armor-broken RED valence must differ from the wound AMBER",
        );
    }
}
