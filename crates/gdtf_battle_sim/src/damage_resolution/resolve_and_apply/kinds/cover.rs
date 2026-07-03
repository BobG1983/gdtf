//! The **cover** struck-kind — the GTW-364 cover-hit arm of the E3.9 fold
//! (`docs/combat/resolution.md` §3) and its per-kind verdict payload
//! ([`CoverVerdict`]), owned here per the GTW-573 one-module-per-kind layout.
//!
//! Cover uses the **same armor/damage model as a ganger**: the SAME
//! [`resolve_hit`] formula resolves the hit damage against the struck cover's own
//! armor stats, that HP is spent through the EXISTING [`CoverLedger::deplete_cover`],
//! and a depletion to zero records the destroyed `(cell, level)` on the verdict (the
//! fire path bridges it to a buffered
//! [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) message). Takes **no**
//! RNG draw on either stream — the cover-vs-armor formula is deterministic
//! (replay-safe).

use crate::{
    armor::{ArmorFloor, ArmorIntegrity, ArmorPiece, ArmorType},
    cover::{CoverDamage, CoverEntry, CoverEvent, CoverLedger},
    matchup::Matchup,
    metric::CellLevel,
    resolve_and_apply::report::HitVerdict,
    resolve_hit::{HpDamage, resolve_hit},
    tuning::CombatTuning,
    weapon::WeaponStats,
};

/// The **cover verdict** — what one round that struck a piece of cover froze: the
/// destroyed `(cell, level)` iff this round depleted the cover's structural HP to
/// zero, else `None` (the round chipped the cover — HP fell, the piece still stands).
///
/// The GTW-573 per-kind payload of [`HitVerdict::Cover`]: a damaged-not-destroyed
/// cover hit is `CoverVerdict { destroyed: None }` — a REAL cover-hit verdict, not a
/// no-effect fold (the presenter pops its "Cover hit" chip off exactly this shape).
/// The fire path bridges a `Some` into the buffered
/// [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverVerdict {
    /// The `(cell, level)` of the cover this round DESTROYED — `Some` only when the
    /// hit depleted that cell's structural HP to zero.
    pub destroyed: Option<CellLevel>,
}

/// The bare-flesh-shaped [`ArmorPiece`] a piece of cover's hit resolves against —
/// the cover's own [`ArmorProtection`](crate::armor::ArmorProtection) /
/// [`ArmorHardness`](crate::armor::ArmorHardness) (its `floor` 0 — cover has no
/// "still bruises" floor; its `integrity` 0 — the formula never reads it; its type
/// [`ArmorType::DEFAULT`] — cover has no matchup-wheel node, so the matchup is forced
/// [`Matchup::Neutral`] below).
///
/// Cover uses the **same armor/damage model as a ganger** (`docs/combat/resolution.md`
/// §3), so the cover hit runs the EXACT same [`resolve_hit`] formula the ganger path
/// runs — only the armor stats it resolves against come from the struck
/// [`CoverEntry`] instead of a worn piece.
///
/// `pub(crate)` so the §7 melee cover-smash path
/// ([`resolve_structural_melee`](crate::melee::resolve_structural_melee), GTW-508)
/// resolves against the SAME shared armor-piece shape — it is imported there, never
/// re-defined, so the two cover-hit paths cannot drift.
pub(crate) const fn cover_armor_piece(entry: &CoverEntry) -> ArmorPiece {
    ArmorPiece::new(
        ArmorFloor::new(0),
        entry.armor_protection,
        ArmorIntegrity::new(0),
        entry.armor_hardness,
        ArmorType::DEFAULT,
    )
}

/// Convert a resolved [`HpDamage`] into the [`CoverDamage`] the cover ledger spends —
/// the shared HP-loss → cover-HP conversion both the ranged cover hit ([`fold`])
/// and the §7 melee cover-smash ([`resolve_structural_melee`](crate::melee::resolve_structural_melee),
/// GTW-508) route through.
///
/// [`HpDamage`] is a signed `i32` (it can read negative pre-floor, though the cover
/// formula's floor `0` keeps it `≥ 0` here); a fully-soaked hit removes no HP, so the
/// conversion clamps at zero — cover HP is a non-negative pool. `pub(crate)` so the melee
/// path reuses this ONE conversion instead of copying the `u32::try_from(... .max(0))`
/// glue (GTW-508 C1 — shared, not duplicated).
pub(crate) fn cover_damage_from_hp(hp_damage: HpDamage) -> CoverDamage {
    CoverDamage::new(u32::try_from((*hp_damage).max(0)).unwrap_or(0))
}

/// Fold a **[`ShotKind::Cover`](crate::resolve_coarse::ShotKind::Cover)** outcome —
/// spend the shot's damage against the struck cover and freeze the [`CoverVerdict`]
/// (the fire→deplete→message bridge's model half; `docs/combat/resolution.md` §3).
///
/// The pipeline (C1 → C2):
///
/// 1. **Damage (C1)** — the SAME [`resolve_hit`] formula the ganger path uses, run
///    against the cover's own armor stats ([`cover_armor_piece`]) under
///    [`Matchup::Neutral`] (cover has no wheel node). Its
///    [`HpDamage`](crate::resolve_hit::HpDamage) is the HP the hit removes — reused
///    verbatim, NOT a new parallel formula.
/// 2. **Deplete (C2)** — that HP, converted to a [`CoverDamage`] (clamped at zero —
///    a fully-soaked hit removes no HP), is spent via the EXISTING
///    [`CoverLedger::deplete_cover`]. The prototype is the struck `entry` (so a
///    never-before-hit piece lazy-seeds at its authored `max_hp`); HP bookkeeping and
///    destruction detection are owned by `deplete_cover`, never re-implemented here.
///
/// Returns [`HitVerdict::Cover`] carrying the destroyed `(cell, level)` on a
/// [`CoverEvent::Destroyed`], `None` on a [`CoverEvent::Damaged`]. The `at` is the
/// struck outcome's cell/level (the cover the round stopped on). Takes **no** RNG
/// draw on either stream (deterministic, replay-safe — pinned by
/// `test::draw_discipline`).
pub(in crate::damage_resolution::resolve_and_apply) fn fold(
    entry: &CoverEntry,
    at: CellLevel,
    weapon: WeaponStats<'_>,
    cover: &mut CoverLedger,
    tuning: &CombatTuning,
) -> HitVerdict {
    // (1) C1 — the per-hit damage formula, REUSED verbatim (the ganger path's E3.3),
    //     against the cover's own armor stats under Neutral (cover has no wheel node).
    let piece = cover_armor_piece(entry);
    let hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        Matchup::Neutral,
        tuning,
    );

    // The resolved HP-loss damage → a CoverDamage via the SHARED conversion (clamped at
    // zero — cover HP is a non-negative pool); the melee cover-smash routes through the
    // SAME helper (GTW-508 C1 — one conversion, not two).
    let removed = cover_damage_from_hp(hit.hp_damage);

    // (2) C2 — spend it through the EXISTING ledger API (HP bookkeeping + destruction
    //     detection owned there). The prototype is the struck entry, so a never-hit
    //     piece lazy-seeds at its authored max_hp before this hit deducts.
    let destroyed = match cover.deplete_cover(at, removed, *entry, tuning) {
        CoverEvent::Destroyed(cell) => Some(cell),
        CoverEvent::Damaged(_) => None,
    };
    HitVerdict::Cover(CoverVerdict { destroyed })
}
