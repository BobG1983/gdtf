//! The §7 melee **cover-smash** resolution verb — the pure, render-free function that
//! resolves an UNCONTESTED melee strike against an adjacent inert STRUCTURE (a Cover or
//! Wall cell) (GTW-508, child GTW-37d of GTW-37).
//!
//! `docs/combat/resolution.md` §7 designs a melee attack against a GANGER as an opposed
//! Fight roll (GTW-506/507). A structure does **not** defend — it is inert — so a
//! melee strike on an adjacent Cover/Wall cell is resolved as an UNCONTESTED blow: there
//! is NO opposed Fight roll and NO [`FightRng`](crate::rng::FightRng) draw. The strike
//! deterministically applies MULTIPLIED weapon melee damage to that structure's HP
//! through the EXISTING cover ledger.
//!
//! ## What is reused (nothing is reimplemented)
//!
//! Every cover-hit primitive the ranged shoot-the-cover path uses is IMPORTED here, not
//! copied — the two paths cannot drift (GTW-508 C1):
//!
//! - The cover armor-piece shape (`cover_armor_piece`, `resolve_and_apply::fold`) — the
//!   struck cover's OWN [`ArmorProtection`](crate::armor::ArmorProtection) /
//!   [`ArmorHardness`](crate::armor::ArmorHardness) under [`Matchup::Neutral`] (cover has
//!   no matchup-wheel node), the ONE definition the ranged `apply_cover_hit` resolves
//!   against too.
//! - The §5 per-hit damage formula ([`resolve_hit`](crate::resolve_hit::resolve_hit)) —
//!   run against that shared armor-piece shape, EXACTLY the way the ranged shoot-the-cover
//!   path resolves a cover hit (`docs/combat/resolution.md` §3: cover "uses the same
//!   armor/damage model as a ganger").
//! - The HP-loss → cover-HP conversion (`cover_damage_from_hp`, `resolve_and_apply::fold`)
//!   — the shared, clamped [`HpDamage`](crate::resolve_hit::HpDamage) →
//!   [`CoverDamage`](crate::cover::CoverDamage) glue the ranged `apply_cover_hit` routes
//!   through, imported, never re-written.
//! - The §7 melee multiplier step ([`apply_melee_multiplier`](crate::melee::apply_melee_multiplier),
//!   GTW-506) — scaling the resolved [`HitResult`](crate::resolve_hit::HitResult) by the
//!   FORK-4a structural multiplier (`mult_max` — see [`StructuralMult`]). This is the ONE
//!   step the ranged path lacks (a structural blow is multiplied); it sits BETWEEN the
//!   shared resolve-hit and the shared conversion, so the glue stays shared and only the
//!   melee-specific multiply is added.
//! - The cover/structure HP ledger ([`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover),
//!   the `apply_cover_hit` bridge) — spends the resolved HP and, on depletion to zero,
//!   returns [`CoverEvent::Destroyed`] carrying the `(cell, level)`. HP bookkeeping and
//!   destruction detection are OWNED there, never re-implemented here.
//!
//! ## FORK 4a — the uncontested structural multiplier
//!
//! A stationary structure offers no defence, so the strike lands at the TOP of the
//! contested damage range: the multiplier is [`MeleeTuning::mult_max`](crate::tuning::MeleeTuning)
//! — the exact ceiling a maximally-dominant opposed hit would clamp to. This REUSES the
//! existing tuning constant and introduces NO new magic number (the fork is logged on
//! GTW-508). The verb reads it through [`StructuralMult::from_tuning`].
//!
//! Pure model logic: no systems, no `&mut World`, no ECS trigger, no pixel. The owning
//! [`dispatch_melee`](crate::acts::dispatch_melee) system (GTW-508) gates 8-adjacency to
//! the structure cell and calls this verb once, then bridges a returned
//! [`CoverEvent::Destroyed`] to the existing
//! [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) signal (the GTW-386 FX).

use crate::{
    cover::{CoverEntry, CoverEvent, CoverLedger},
    matchup::Matchup,
    melee::{MeleeDamageMult, MeleeWeaponHit, apply_melee_multiplier},
    metric::CellLevel,
    resolve_and_apply::{cover_armor_piece, cover_damage_from_hp},
    resolve_hit::resolve_hit,
    tuning::{CombatTuning, MeleeTuning},
};

/// The **uncontested structural damage multiplier** a melee cover-smash applies — FORK
/// 4a's `mult_max` (`docs/combat/resolution.md` §7; GTW-508).
///
/// An inert structure does not defend, so there is no opposed-Fight margin to scale the
/// blow by; the strike lands at the TOP of the contested damage range, i.e. the exact
/// [`MeleeTuning::mult_max`](crate::tuning::MeleeTuning) ceiling a maximally-dominant
/// opposed melee hit would clamp to. A distinct newtype from
/// [`MeleeDamageMult`](crate::melee::MeleeDamageMult) so the uncontested structural
/// multiplier is never confused with a rolled contested one (no-bare-types rule 3),
/// though it converts INTO a [`MeleeDamageMult`] to reuse the shared
/// [`apply_melee_multiplier`](crate::melee::apply_melee_multiplier) function verbatim. It
/// reuses the existing `mult_max` tuning value and pins NO new magic number. A domain
/// math value — **zero pixels**; private inner + a named constructor (no public `Deref`
/// — read it only through [`as_melee_mult`](StructuralMult::as_melee_mult)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructuralMult(f32);

impl StructuralMult {
    /// The FORK-4a structural multiplier for `tuning` — its
    /// [`MeleeTuning::mult_max`](crate::tuning::MeleeTuning), the top of the contested
    /// damage range (an inert structure offers no defence).
    ///
    /// Reads the EXISTING `mult_max` constant, introducing no new tunable; if `MeleeTuning`
    /// ever grows a dedicated structural field, this is the single place to switch the
    /// source (the fork's documented change point).
    #[must_use]
    pub fn from_tuning(tuning: &MeleeTuning) -> Self {
        Self(*tuning.mult_max)
    }

    /// This multiplier as the shared [`MeleeDamageMult`](crate::melee::MeleeDamageMult)
    /// the [`apply_melee_multiplier`](crate::melee::apply_melee_multiplier) function consumes.
    ///
    /// The one read-out — the structural verb scales the resolved cover
    /// [`HitResult`](crate::resolve_hit::HitResult) through the SAME multiplier function the
    /// ganger path uses, so no parallel scaling math exists.
    #[must_use]
    pub const fn as_melee_mult(self) -> MeleeDamageMult {
        MeleeDamageMult::new(self.0)
    }
}

/// Resolve ONE uncontested §7 melee strike onto the adjacent STRUCTURE at `at` — the
/// cover-smash synthesis (GTW-508).
///
/// UNCONTESTED: a structure is inert, so there is **NO opposed-Fight roll and NO
/// [`FightRng`](crate::rng::FightRng) draw** (nor any `ShotRng` / `SeverityRng` draw — a
/// structure has no body-part location and no wound severity). The blow deterministically:
///
/// 1. **Damage** — the SAME [`resolve_hit`] formula the ganger / ranged-cover paths use,
///    run against the struck cover's own armor stats (the SHARED `cover_armor_piece` shape,
///    imported from the ranged path) under [`Matchup::Neutral`] (cover has no wheel node).
///    REUSED verbatim — no parallel formula.
/// 2. **Multiply (FORK 4a)** — the resolved [`HitResult`](crate::resolve_hit::HitResult)
///    is scaled by the [`StructuralMult`] (`mult_max`) through the SAME
///    [`apply_melee_multiplier`](crate::melee::apply_melee_multiplier) function the contested
///    ganger path uses, so the strike lands at the top of the melee damage range.
/// 3. **Deplete** — that scaled HP-loss, converted to a [`CoverDamage`](crate::cover::CoverDamage)
///    (clamped at zero — a fully-soaked hit removes no HP; cover HP is a non-negative pool), is
///    spent via the EXISTING [`CoverLedger::deplete_cover`]. The prototype is the struck `entry` (so a
///    never-before-hit piece lazy-seeds at its authored `max_hp`); HP bookkeeping and the
///    destruction verdict are owned by `deplete_cover`, never re-implemented here.
///
/// Returns the [`CoverEvent`] (`Damaged` / `Destroyed`) `deplete_cover` produced — the
/// owning [`dispatch_melee`](crate::acts::dispatch_melee) bridges a `Destroyed` into the
/// existing [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) signal (the GTW-386
/// FX). Same tuning + same struck HP ⇒ the same [`CoverEvent`] every run (deterministic —
/// it takes NO RNG draw, so it cannot perturb any seeded stream; the replay-safe property
/// C5(c) asserts).
///
/// # Arguments
///
/// - `weapon` — the wielded melee weapon's §5 damage stats (the [`MeleeWeaponHit`]
///   borrow-view `dispatch_melee` assembles).
/// - `entry` — the struck cover's [`CoverEntry`] prototype (its armor stats + authored
///   `max_hp`), the lazy-seed prototype for `deplete_cover`.
/// - `at` — the `(cell, level)` of the struck structure.
/// - `cover` — the model [`CoverLedger`], the single authoritative cover-HP writer.
/// - `tuning` — the [`CombatTuning`] the §5 formula reads + the source of the FORK-4a
///   `mult_max`.
#[must_use]
pub fn resolve_structural_melee(
    weapon: MeleeWeaponHit<'_>,
    entry: &CoverEntry,
    at: CellLevel,
    cover: &mut CoverLedger,
    tuning: &CombatTuning,
) -> CoverEvent {
    // (1) §5 damage — the per-hit formula, REUSED verbatim (the ganger / ranged-cover
    //     path's E3.3), against the cover's own armor under Neutral (cover has no wheel
    //     node). NO opposed roll precedes it — a structure is inert, so no FightRng draw.
    let piece = cover_armor_piece(entry);
    let raw_hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        Matchup::Neutral,
        tuning,
    );

    // (2) FORK 4a — scale by the uncontested structural multiplier (mult_max) through the
    //     SAME melee multiplier function the contested ganger path uses (GTW-506).
    let mult = StructuralMult::from_tuning(&tuning.melee).as_melee_mult();
    let hit = apply_melee_multiplier(raw_hit, mult);

    // The scaled HP-loss → a CoverDamage via the SHARED conversion the ranged
    // `apply_cover_hit` routes through (clamped at zero — cover HP is a non-negative pool);
    // imported, not copied (GTW-508 C1).
    let removed = cover_damage_from_hp(hit.hp_damage);

    // (3) Spend it through the EXISTING ledger API (HP bookkeeping + destruction detection
    //     owned there). The prototype is the struck entry, so a never-hit piece lazy-seeds
    //     at its authored max_hp before this hit deducts, and PERSISTS across strikes.
    cover.deplete_cover(at, removed, *entry, tuning)
}
