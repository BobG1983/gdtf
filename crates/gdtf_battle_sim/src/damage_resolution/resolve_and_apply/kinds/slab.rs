//! The **slab** struck-kind — the GTW-365 slab-hit arm of the E3.9 fold
//! (`docs/combat/resolution.md` §3.1; user-ruled 2026-06-22) and its per-kind verdict
//! payload ([`SlabVerdict`]), owned here per the GTW-573 one-module-per-kind layout —
//! the slab mirror of [`cover`](super::cover).
//!
//! A floor/roof slab has its **own HP + armor**, so the SAME
//! [`resolve_hit`] formula resolves the hit damage against the struck slab's own
//! armor stats, that HP is spent through the EXISTING [`SlabLedger::deplete_slab`],
//! and a depletion to zero records the destroyed `(cell, level)` on the verdict (the
//! fire path bridges it to a buffered
//! [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed) message). Takes **no** RNG
//! draw on either stream — the slab-vs-armor formula is deterministic (replay-safe).

use crate::{
    armor::{ArmorFloor, ArmorIntegrity, ArmorPiece, ArmorType},
    matchup::Matchup,
    metric::CellLevel,
    resolve_and_apply::report::HitVerdict,
    resolve_hit::resolve_hit,
    slab::{SlabDamage, SlabEntry, SlabEvent, SlabLedger},
    tuning::CombatTuning,
    weapon::WeaponStats,
};

/// The **slab verdict** — what one round that struck a floor/roof slab froze: the
/// destroyed `(cell, level)` iff this round depleted the slab's structural HP to
/// zero, else `None` (the round chipped the slab — HP fell, the slab still stands).
///
/// The GTW-573 per-kind payload of [`HitVerdict::Slab`] — the slab mirror of
/// [`CoverVerdict`](super::cover::CoverVerdict) (a distinct named type: the two
/// bridge to different destruction messages and must never be swapped). The fire path
/// bridges a `Some` into the buffered
/// [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed) message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabVerdict {
    /// The `(cell, level)` of the slab this round DESTROYED — `Some` only when the
    /// hit depleted that slab's structural HP to zero.
    pub destroyed: Option<CellLevel>,
}

/// The bare-flesh-shaped [`ArmorPiece`] a floor/roof slab's hit resolves against — the
/// slab's own [`ArmorProtection`](crate::armor::ArmorProtection) /
/// [`ArmorHardness`](crate::armor::ArmorHardness) (its `floor` 0 / `integrity` 0 /
/// type [`ArmorType::DEFAULT`] forced [`Matchup::Neutral`], the same shape
/// [`cover_armor_piece`](super::cover::cover_armor_piece) builds for cover).
///
/// A slab uses the **same armor/damage model as a ganger and cover**
/// (`docs/combat/resolution.md` §3.1; user-ruled 2026-06-22), so the slab hit runs the
/// EXACT same [`resolve_hit`] formula the ganger / cover path runs — only the armor
/// stats it resolves against come from the struck [`SlabEntry`].
const fn slab_armor_piece(entry: &SlabEntry) -> ArmorPiece {
    ArmorPiece::new(
        ArmorFloor::new(0),
        entry.armor_protection,
        ArmorIntegrity::new(0),
        entry.armor_hardness,
        ArmorType::DEFAULT,
    )
}

/// Fold a **[`ShotKind::Slab`](crate::resolve_coarse::ShotKind::Slab)** outcome —
/// spend the shot's damage against the struck slab and freeze the [`SlabVerdict`]
/// (the slab mirror of [`cover::fold`](super::cover::fold);
/// `docs/combat/resolution.md` §3.1).
///
/// The pipeline:
///
/// 1. **Damage** — the SAME [`resolve_hit`] formula the ganger / cover path uses, run
///    against the slab's own armor stats ([`slab_armor_piece`]) under
///    [`Matchup::Neutral`] (a slab has no wheel node). Its
///    [`HpDamage`](crate::resolve_hit::HpDamage) is the HP the hit removes — reused
///    verbatim, NOT a new parallel formula.
/// 2. **Deplete** — that HP, converted to a [`SlabDamage`] (clamped at zero — a
///    fully-soaked hit removes no HP), is spent via the EXISTING
///    [`SlabLedger::deplete_slab`]. The prototype here is the NO-PANIC FALLBACK for a
///    slab struck with no authored entry (an out-of-bounds / non-authored cell) —
///    `SlabLedger::entry_seeded` returns an eagerly-inserted (GTW-396 Decision C)
///    entry UNCHANGED and ignores the fallback, so authored HP is always honored; the
///    code-only `SlabDefaults::FALLBACK` (NOT a tuning leaf — removed in GTW-396)
///    fires ONLY for an unauthored strike. HP bookkeeping + destruction detection are
///    owned by `deplete_slab`, never re-implemented here. PERSISTENT across strikes:
///    a second hit reads the reduced `current_hp` from the map.
///
/// Returns [`HitVerdict::Slab`] carrying the destroyed `(cell, level)` on a
/// [`SlabEvent::Destroyed`], `None` on a [`SlabEvent::Damaged`]. Takes **no** RNG
/// draw on either stream (deterministic, replay-safe — pinned by
/// `test::draw_discipline`).
pub(in crate::damage_resolution::resolve_and_apply) fn fold(
    at: CellLevel,
    weapon: WeaponStats<'_>,
    slab: &mut SlabLedger,
    tuning: &CombatTuning,
) -> HitVerdict {
    /// Code-only fallback for an unauthored slab strike (no registry entry).
    /// NOT the tuning leaf (removed in GTW-396) — a no-panic backstop only.
    const SLAB_FALLBACK_DEFAULTS: crate::tuning::SlabDefaults =
        crate::tuning::SlabDefaults::FALLBACK;
    let prototype = SlabLedger::prototype_for(at, &SLAB_FALLBACK_DEFAULTS);

    // (1) The per-hit damage formula, REUSED verbatim (the ganger / cover path's E3.3),
    //     against the slab's own armor stats under Neutral (a slab has no wheel node).
    let piece = slab_armor_piece(&prototype);
    let hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        Matchup::Neutral,
        tuning,
    );

    // The resolved HP-loss damage → a SlabDamage. HpDamage is a signed i32 (floor 0 keeps
    // it ≥ 0 here); a fully-soaked hit removes no HP, so clamp the conversion at zero
    // (slab HP is a non-negative pool).
    let removed = SlabDamage::new(u32::try_from((*hit.hp_damage).max(0)).unwrap_or(0));

    // (2) Spend it through the EXISTING ledger API (HP bookkeeping + destruction detection
    //     owned there).
    let destroyed = match slab.deplete_slab(at, removed, prototype) {
        SlabEvent::Destroyed(cell) => Some(cell),
        SlabEvent::Damaged(_) => None,
    };
    HitVerdict::Slab(SlabVerdict { destroyed })
}
