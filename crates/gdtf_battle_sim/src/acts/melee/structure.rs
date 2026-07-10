//! The uncontested GTW-508 cover-smash melee arm — gate 8-adjacency, spend the
//! same fight-mode TU, and smash multiplied weapon damage through the cover ledger.

use bevy::prelude::{MessageWriter, Query};

use super::{MeleeGrids, snapshot::AttackerSnapshot};
use crate::{
    acts::{downed::is_8_adjacent, request::MeleeResolved},
    cover::CoverEvent,
    ganger::{Position, Tu},
    melee::resolve_structural_melee,
    metric::CellLevel,
    occupancy_sync::CoverDestroyed,
    tu::spend_tu,
};

/// Resolve ONE melee-vs-structure request — the UNCONTESTED §7 cover-smash path (GTW-508),
/// extracted from [`dispatch_melee`](super::dispatch_melee)'s target branch (GTW-508 C6 — file
/// size cap).
///
/// Gates ONLY 8-adjacency to the struck cell (LOS to an immediately-adjacent structure is
/// trivially satisfied — NO spurious LOS block, the C3 ruling), spends the same fight-mode TU,
/// and calls [`resolve_structural_melee`] — multiplied (`mult_max`, FORK 4a) weapon damage
/// through the EXISTING [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover).
/// There is **NO opposed roll and NO `FightRng`/`ShotRng`/`SeverityRng` draw** — a structure is
/// inert. On a lethal smash it fires the EXISTING [`CoverDestroyed`] signal (the GTW-386 FX);
/// on either outcome it emits the [`MeleeResolved`] strike-glyph (a structure never dodges, so
/// there is no connect gate). Fail-closed on a failed adjacency gate / missing Tu pool.
pub(super) fn resolve_structure_melee(
    attacker: &AttackerSnapshot<'_>,
    at: CellLevel,
    tu_q: &mut Query<&mut Tu>,
    grids: &mut MeleeGrids,
    resolved: &mut MessageWriter<MeleeResolved>,
    cover_destroyed: &mut MessageWriter<CoverDestroyed>,
    deaths: &mut MessageWriter<crate::effects::on_death::OnDeathOccurred>,
) {
    // Gate — 8-adjacency to the struck STRUCTURE cell (reuse `is_8_adjacent` over the attacker's
    // Position vs a Position at the target cell). LOS to an immediately-adjacent structure is
    // trivially satisfied, so NO LOS block is applied (a spurious LOS gate would reject the very
    // cover the attacker stands beside — the C3 ruling).
    if !*is_8_adjacent(attacker.position, Position::new(at)) {
        return;
    }

    // The struck cover's prototype — the ledger's stored entry if present, else a lazily-seeded
    // intact wall. `peek` distinguishes an already-registered cell from an unregistered one;
    // either way the smash resolves against a defined entry (the ledger owns the lazy seed). An
    // unregistered cell has no authored HP/armor, so it seeds from the shared no-panic default (a
    // strike on an out-of-bounds / empty cell still resolves defined bookkeeping, never a panic).
    let prototype = grids
        .cover
        .peek(&at)
        .copied()
        .unwrap_or(STRUCTURE_SMASH_FALLBACK);

    // Spend the fight-mode TU off the attacker (saturating) — a swing at a structure costs TU
    // exactly like a swing at a ganger (fail-closed on a missing Tu pool).
    let Ok(mut attacker_tu) = tu_q.get_mut(attacker.entity) else {
        return;
    };
    spend_tu(&mut attacker_tu, attacker.tu_cost);

    // The UNCONTESTED smash — multiplied (mult_max) weapon damage through the EXISTING ledger
    // `deplete_cover`. NO opposed roll, NO FightRng/ShotRng/SeverityRng draw.
    let event = resolve_structural_melee(
        attacker.weapon,
        &prototype,
        at,
        &mut grids.cover,
        &grids.tuning,
    );

    // On a lethal smash, fire the EXISTING cover-destroyed signal (the presenter's GTW-386
    // rubble-burst FX reacts to it verbatim).
    if let CoverEvent::Destroyed(cell) = event {
        cover_destroyed.write(CoverDestroyed::new(cell));
        // GTW-547: a destroyed piece of cover ALSO emits the terminal-death signal (keyed by
        // its cell — cover is not an entity, so Entity::PLACEHOLDER) so `resolve_on_death` fans
        // the cover tile's authored on-death effect (the ranged cover-destroy bridge mirror).
        deaths.write(crate::effects::on_death::OnDeathOccurred::cover(cell));
    }
    // Emit the strike-glyph at the struck structure cell (a structural smash always lands — a
    // structure never dodges — so unlike the ganger path there is no connect gate here).
    resolved.write(MeleeResolved::new(at, attacker.strike_damage_type));
}

/// The fallback [`CoverEntry`](crate::cover::CoverEntry) prototype a melee cover-smash resolves
/// against when the struck cell has NO ledger entry yet (an unregistered / unauthored cell) — a
/// no-panic backstop, NOT authored data.
///
/// A registered cell (a real wall / prop) is read through
/// [`CoverLedger::peek`](crate::cover::CoverLedger::peek); this fallback fires ONLY for a strike
/// on a cell that was never registered or hit. It seeds an intact, ARMORLESS piece (protection /
/// hardness `0`, a small `max_hp`, a `Low` band) so the smash resolves defined bookkeeping —
/// [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover) ignores this
/// prototype whenever an authored entry already exists (its `entry_seeded` returns the stored
/// entry unchanged), so authored HP/armor is always honoured; this only backstops an
/// unregistered strike (the ranged `SLAB_FALLBACK_DEFAULTS` precedent). A code-only const — no
/// tuning read, no pinned balance magnitude a test asserts.
const STRUCTURE_SMASH_FALLBACK: crate::cover::CoverEntry = crate::cover::CoverEntry::seeded(
    crate::cover::CoverHp::new(1),
    crate::cover::HeightBand::Low,
    crate::armor::ArmorProtection::new(0),
    crate::armor::ArmorHardness::new(0),
);
