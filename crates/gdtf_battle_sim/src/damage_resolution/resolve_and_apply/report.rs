//! The frozen value types of the E3.9 fold — the [`TargetGanger`] borrow-view the
//! fold mutates, and the `Copy` [`HitReport`] / [`AppliedDamage`] records it
//! returns. No-bare-types, no pixel: every field is a named domain newtype.

use crate::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart},
    armor_wear::{ArmorBroken, ArmorWorn},
    cover::CoverLedger,
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::RolledInjury,
    matchup::Matchup,
    metric::{Cell, CellLevel},
    resolve_coarse::ShotKind,
    resolve_hit::HitResult,
    severity::Severity,
    slab::SlabLedger,
    surface::GroundDamage,
};

/// The **bundle of one target ganger's battle state** [`resolve_and_apply`](super::resolve_and_apply)
/// folds a hit onto — the four `&mut` battle surfaces a hit can change, plus the
/// two read attribute stats the severity roll needs.
///
/// Grouping these into one named struct keeps
/// [`resolve_and_apply`](super::resolve_and_apply) under clippy's argument-count
/// gate (the [`GangerHitTarget`](crate::apply_hit::GangerHitTarget) /
/// [`SeverityInputs`](crate::severity::SeverityInputs) precedent). The mutable
/// borrows are exactly the [`GangerHitTarget`](crate::apply_hit::GangerHitTarget)
/// set (assembled from this bundle when
/// [`apply_hit`](crate::apply_hit::apply_hit) runs); the two reads
/// ([`Toughness`] / [`Luck`]) are the E3.0 defender attribute components fed to the
/// severity roll. Every field is an existing named domain component (no-bare-types).
/// The caller (a Bevy system, or the E4 `fire()` act) assembles this from the
/// target entity's components.
pub struct TargetGanger<'a> {
    /// The target's hit-points pool — the HP loss subtracts from it (always).
    pub hp:        &'a mut Hp,
    /// The target's Wounds (life) pool — the severity tier spends from it.
    pub wounds:    &'a mut Wounds,
    /// The target's terminal life state — the gates set it; corpse-skip reads it.
    pub life:      &'a mut LifeState,
    /// The struck location's worn-armor **piece** — its read stats + the mutable
    /// integrity the hit wears in place (since GTW-323 / ADR-0004 the piece is a
    /// related entity resolved from `ganger → Wears → the BodyPart-tagged piece`, not a
    /// `WornArmor` array slot). `None` when the struck location wears no piece (a
    /// missing-piece defensive fold → bare flesh).
    pub piece:     Option<StruckPiece<'a>>,
    /// The target's inflicted-wound record (GTW-279) — each registered wound appends
    /// its tier + struck part here (the additive presentation record for GTW-278).
    pub inflicted: &'a mut InflictedWounds,
    /// The target's Toughness — the defender's severity-mitigation term (E3.0, read).
    pub toughness: Toughness,
    /// The target's Luck — extends the severity roll's floor down (E3.0, read).
    pub luck:      Luck,
}

/// The **struck worn-armor piece** borrow-view the fold resolves and applies a hit
/// against — the four read-only stats (consumed by the matchup + the per-hit damage
/// formula) plus the one **mutable** [`ArmorIntegrity`] the wear degrades (GTW-323 /
/// ADR-0004).
///
/// The piece-entity replacement for the old `WornArmor` array-slot access: the caller
/// (the E4 `fire()` path) resolves `ganger → Wears → the BodyPart-tagged piece` and
/// assembles this from that piece entity's [`crate::armor::PieceArmorMut`] query row.
/// The struck-vs-bare-flesh branch reads [`protects`](StruckPiece::protects) (the
/// piece integrity `> 0`); the wear mutates [`integrity`](StruckPiece::integrity) in
/// place. Every read field is a `Copy` named domain newtype (no-bare-types).
pub struct StruckPiece<'a> {
    /// The piece's minimum-damage floor (read).
    pub floor:      ArmorFloor,
    /// The piece's damage-soak protection (read).
    pub protection: ArmorProtection,
    /// The piece's penetration-ignoring hardness (read; does not degrade).
    pub hardness:   ArmorHardness,
    /// The piece's matchup-wheel node (read).
    pub armor_type: ArmorType,
    /// The piece's durability — the **one mutable** wear field (degrades per hit).
    pub integrity:  &'a mut ArmorIntegrity,
}

impl StruckPiece<'_> {
    /// Whether this worn piece still **protects** — its [`ArmorIntegrity`] is strictly
    /// above zero (`weapons-and-armor.md` §"Per-hit resolution" step 3: "useless at
    /// `≤ 0`"). The struck-vs-bare-flesh gate the fold reads (mirrors the old
    /// `WornArmor::protects`).
    #[must_use]
    pub fn protects(&self) -> bool {
        **self.integrity > 0
    }

    /// The piece's current [`ArmorIntegrity`] read **by value** — the read the
    /// per-hit damage formula assembles into its [`crate::armor::ArmorPiece`]
    /// (separate from the `&mut` wear path, so the read borrows immutably).
    #[must_use]
    pub const fn integrity_value(&self) -> ArmorIntegrity {
        *self.integrity
    }
}

/// The **world surfaces a shot can deplete** — the two model HP ledgers
/// [`resolve_and_apply`](super::resolve_and_apply) spends on a structural hit, bundled
/// into one mutable borrow-view so the fold stays under clippy's argument-count gate
/// (the [`TargetGanger`] / `BattleGrids` grouping precedent).
///
/// A [`ShotKind::Cover`](crate::resolve_coarse::ShotKind::Cover) outcome spends
/// [`cover`](StruckSurfaces::cover); a
/// [`ShotKind::Slab`](crate::resolve_coarse::ShotKind::Slab) outcome (GTW-365) spends
/// [`slab`](StruckSurfaces::slab) — a hit touches **exactly one** of the two (its
/// `ShotKind` selects it), and a ganger / ground / miss touches neither. Each field is an
/// existing named model resource (no-bare-types); the struct is a transparent mutable
/// borrow record, not itself a wrapped domain scalar. The caller (the E4 `fire()` path)
/// reborrows its `&mut CoverLedger` / `&mut SlabLedger` into this each round.
pub struct StruckSurfaces<'a> {
    /// The model cover ledger — spent (HP depleted) on a
    /// [`ShotKind::Cover`](crate::resolve_coarse::ShotKind::Cover) hit (GTW-364).
    pub cover: &'a mut CoverLedger,
    /// The model slab ledger — spent (HP depleted) on a
    /// [`ShotKind::Slab`](crate::resolve_coarse::ShotKind::Slab) hit (GTW-365).
    pub slab:  &'a mut SlabLedger,
}

/// The **ground-accrual verdict** of a [`HitReport`] — the [`Cell`] a round struck the
/// ground at and the [`GroundDamage`] it dealt there (GTW-366,
/// `docs/combat/resolution.md` §3.2; user-ruled 2026-06-22).
///
/// A frozen `Copy` record of two named domain newtypes (no bare primitive, no pixel): the
/// ground-plane [`Cell`] the round exited the bottom of the voxel column at, and the
/// round's [`GroundDamage`] (its `weapon_damage`, NOT a constant — GTW-366 C4). The fire
/// path's [`dispatch_fire`](crate::acts::dispatch_fire) bridges it into a buffered
/// [`GroundAccrued`](crate::occupancy_sync::GroundAccrued) message, which
/// [`sync_accrued_ground`](crate::occupancy_sync::sync_accrued_ground) ACCRUES
/// (monotonically) onto the [`SurfaceGrid`](crate::surface::SurfaceGrid). Present
/// (`Some` in [`HitReport::ground_accrued`]) only on a
/// [`ShotKind::Ground`](crate::resolve_coarse::ShotKind::Ground) outcome — the ground is
/// **damaged, never destroyed**, so this records accrual, never destruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroundAccrual {
    /// The ground-plane [`Cell`] the round struck — the accumulator key.
    pub cell:   Cell,
    /// The [`GroundDamage`] the round dealt — its `weapon_damage`, accrued onto the cell.
    pub amount: GroundDamage,
}

impl GroundAccrual {
    /// Build a ground-accrual verdict for the `cell` the round struck and the `amount`
    /// of [`GroundDamage`] it dealt (the round's `weapon_damage`).
    #[must_use]
    pub const fn new(cell: Cell, amount: GroundDamage) -> Self {
        Self { cell, amount }
    }
}

/// The **applied-damage block** of a [`HitReport`] — the resolved damage of a hit
/// that landed on a ganger (`docs/combat/resolution.md` §5 / §6).
///
/// A frozen `Copy` record of named newtypes (no bare primitive, no pixel): the
/// resolved [`Matchup`], the per-hit [`HitResult`], the rolled [`Severity`], the
/// ganger's [`LifeState`] **after** application, and the **mutually-exclusive**
/// armor signals — the `Some(`[`ArmorBroken`]`)` iff this hit broke the struck
/// piece, OR the `Some(`[`ArmorWorn`]`)` iff it reduced the piece short of breaking
/// it (GTW-313). The presenter reads it for FX; it is never mutated after
/// [`resolve_and_apply`](super::resolve_and_apply) returns. Present only when the
/// hit actually landed on a ganger — a non-ganger / corpse-skip / no-part report
/// carries `None` in [`HitReport::applied`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedDamage {
    /// The resolved weapon×armor matchup (E3.2) — [`Matchup::Neutral`] on bare flesh.
    pub matchup:    Matchup,
    /// The resolved per-hit damage / penetration / wear (E3.3).
    pub hit:        HitResult,
    /// The rolled wound severity bucket (E3.4) — the ONE RNG draw's outcome.
    pub severity:   Severity,
    /// The target's [`LifeState`] **after** the hit was applied (E3.6's terminal gates).
    pub life_after: LifeState,
    /// The armor-broken signal iff this hit broke the struck piece (E3.6) — else `None`.
    ///
    /// **Mutually exclusive** with [`worn`](AppliedDamage::worn): at most one of the
    /// two is `Some` per hit (the breaking hit sets `broken`, a wearing hit sets
    /// `worn`, an unaffected hit leaves both `None`).
    pub broken:     Option<ArmorBroken>,
    /// The armor-worn signal iff this hit reduced the struck piece **without**
    /// breaking it (GTW-313) — carries the per-hit integrity `delta`; else `None`.
    ///
    /// **Mutually exclusive** with [`broken`](AppliedDamage::broken) (see above).
    pub worn:       Option<ArmorWorn>,
}

/// The **frozen per-hit report** [`resolve_and_apply`](super::resolve_and_apply)
/// returns — the entire E3.9 fold's verdict (the frozen per-round report the
/// authoritative model hands the view; ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A `Copy` value object of named domain types (no bare primitive, **no pixel** —
/// it carries only damage / wound math, never a screen coordinate). The presenter
/// reads it for FX staging; [`resolve_and_apply`](super::resolve_and_apply) owns no
/// mutation after it is returned.
///
/// - [`kind`](HitReport::kind) — what the shot struck (the
///   [`ShotOutcome`](crate::resolve_coarse::ShotOutcome)'s [`ShotKind`], carrying
///   the struck ganger / cover / surface-cell payload).
/// - [`part`](HitReport::part) — the struck [`BodyPart`], `Some` only for a hit
///   that landed on a ganger.
/// - [`applied`](HitReport::applied) — the [`AppliedDamage`] block, `Some` only
///   for a hit that landed on a ganger; `None` for a non-ganger outcome, a
///   corpse-skip, or a defensively-missing part (a **no-effect** report).
/// - [`cover_destroyed`](HitReport::cover_destroyed) — `Some(cell, level)` ONLY
///   when this round depleted a piece of cover's HP to zero (GTW-364); the fire
///   path's [`dispatch_fire`](crate::acts::dispatch_fire) bridges it into a buffered
///   [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) message (the
///   fire→deplete→message bridge). `None` for every non-destroying outcome (a ganger
///   hit, a non-destroying cover hit, a slab / ground / miss).
/// - [`slab_destroyed`](HitReport::slab_destroyed) — `Some(cell, level)` ONLY when this
///   round depleted a floor/roof slab's HP to zero (GTW-365); the fire path bridges it
///   into a buffered [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed) message
///   (the slab mirror of `cover_destroyed`). `None` for every non-destroying or
///   non-slab outcome.
/// - [`ground_accrued`](HitReport::ground_accrued) — `Some(`[`GroundAccrual`]`)` ONLY when
///   this round struck the ground (GTW-366); the fire path bridges it into a buffered
///   [`GroundAccrued`](crate::occupancy_sync::GroundAccrued) message that
///   [`sync_accrued_ground`](crate::occupancy_sync::sync_accrued_ground) accrues
///   (monotonically) onto the [`SurfaceGrid`](crate::surface::SurfaceGrid). The ground is
///   **damaged, never destroyed**, so this records accrual, never destruction. `None`
///   for every non-ground outcome.
/// - [`injury`](HitReport::injury) — `Some(`[`RolledInjury`]`)` ONLY when this round
///   wounded a ganger with a non-graze, non-fatal [`Severity`] AND the
///   `(part, severity)` injury table rolled a named injury (GTW-438); the fire path
///   bridges it into an [`InjuryInflicted`](crate::acts::InjuryInflicted) message the
///   [`apply_injury`](crate::acts::apply_injury) boundary system drains. `None` for a
///   graze / `Fatal` / corpse-skip / non-ganger outcome, or an empty/missing table (the
///   roll still took its one [`InjuryRng`](crate::rng::InjuryRng) draw — see
///   [`roll_injury`](crate::injuries::roll_injury)).
///
/// NOTE — [`HitReport`] is [`Clone`] but NOT `Copy`: the [`injury`](HitReport::injury)
/// field carries a [`RolledInjury`] (an owned `Vec` of effects + three texts), so the
/// report is cloned (not bit-copied) where it is forwarded (the fire path's per-round
/// emission clones it once into `InjuryInflicted` / `ShotFired`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitReport {
    /// What the shot struck — the [`ShotOutcome`](crate::resolve_coarse::ShotOutcome)'s [`ShotKind`].
    pub kind:            ShotKind,
    /// The struck [`BodyPart`] — `Some` only when the hit landed on a ganger.
    pub part:            Option<BodyPart>,
    /// The applied-damage block — `Some` only when the hit landed on a ganger;
    /// `None` is a no-effect report (non-ganger / corpse-skip / no-part).
    pub applied:         Option<AppliedDamage>,
    /// The `(cell, level)` of a piece of cover this round DESTROYED (GTW-364) — `Some`
    /// only when the cover-hit pipeline depleted that cell's structural HP to zero;
    /// the fire path bridges it to a [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed)
    /// message. `None` for a non-destroying or non-cover outcome.
    pub cover_destroyed: Option<CellLevel>,
    /// The `(cell, level)` of a floor/roof slab this round DESTROYED (GTW-365) — `Some`
    /// only when the slab-hit pipeline depleted that slab's structural HP to zero; the
    /// fire path bridges it to a [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed)
    /// message. `None` for a non-destroying or non-slab outcome.
    pub slab_destroyed:  Option<CellLevel>,
    /// The ground-accrual verdict of a round that struck the GROUND (GTW-366) — `Some`
    /// only on a [`ShotKind::Ground`](crate::resolve_coarse::ShotKind::Ground) outcome,
    /// carrying the struck [`Cell`] + the round's [`GroundDamage`]; the fire path bridges
    /// it to a [`GroundAccrued`](crate::occupancy_sync::GroundAccrued) message that accrues
    /// (monotonically) onto the [`SurfaceGrid`](crate::surface::SurfaceGrid). `None` for a
    /// non-ground outcome. The ground is damaged, never destroyed (purely cosmetic).
    pub ground_accrued:  Option<GroundAccrual>,
    /// The injury this round rolled (GTW-438) — `Some(`[`RolledInjury`]`)` ONLY on a
    /// ganger wound whose non-graze, non-fatal [`Severity`] rolled a named injury from
    /// the `(part, severity)` table; the fire path bridges it to an
    /// [`InjuryInflicted`](crate::acts::InjuryInflicted) message. `None` for a graze /
    /// `Fatal` / corpse-skip / non-ganger outcome, or an empty/missing table (where the
    /// roll still took its one [`InjuryRng`](crate::rng::InjuryRng) draw, then discarded
    /// it — the content-independent stream-alignment property).
    pub injury:          Option<RolledInjury>,
}

impl HitReport {
    /// Build a **no-effect** report for `kind` — no part struck, no damage applied,
    /// no cover / slab destroyed, no ground accrued, and no injury rolled (the
    /// non-ganger non-cover non-slab non-ground, corpse-skip, and defensive-no-part
    /// folds).
    ///
    /// `pub` so the E4.5 `fire()` act (GTW-198) can fold a non-ganger / corpse-skip
    /// round to a no-effect report cross-module without re-deriving the shape. Stays
    /// `const` (every field is a `None` / the `Copy` `kind`), even though the struct now
    /// carries the non-`Copy` [`injury`](HitReport::injury) (a `None` literal is const-OK).
    #[must_use]
    pub const fn no_effect(kind: ShotKind) -> Self {
        Self {
            kind,
            part: None,
            applied: None,
            cover_destroyed: None,
            slab_destroyed: None,
            ground_accrued: None,
            injury: None,
        }
    }
}
