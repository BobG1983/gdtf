//! The frozen value types of the E3.9 fold — the [`TargetGanger`] borrow-view the
//! fold mutates, the [`HitVerdict`] per-kind verdict enum, and the [`HitReport`]
//! record it returns. No-bare-types, no pixel: every field is a named domain newtype.
//!
//! GTW-573: the report's old parallel per-kind `Option` bag (`part` / `applied` /
//! `cover_destroyed` / `slab_destroyed` / `ground_accrued` / `injury` / `dot_applied`,
//! whose mutual exclusivity lived only in prose) is replaced by the ONE closed
//! [`HitVerdict`] enum — exactly one per-kind payload per report, illegal
//! combinations unrepresentable. Each variant's payload type lives in its own
//! [`kinds`](super::kinds) module.

use bevy::prelude::{Deref, Entity};

use crate::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType},
    cover::CoverLedger,
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    resolve_and_apply::kinds::{
        cover::CoverVerdict, ganger::GangerVerdict, ground::GroundAccrual, slab::SlabVerdict,
    },
    resolve_coarse::ShotKind,
    slab::SlabLedger,
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

/// Whether a struck worn-armor piece still **protects** — its [`ArmorIntegrity`] is
/// strictly above zero (`weapons-and-armor.md` §"Per-hit resolution" step 3: "useless
/// at `≤ 0`").
///
/// A named domain answer (no bare `bool`): it drives the struck-vs-bare-flesh branch
/// of the fold — a still-protecting piece soaks/matchups, a broken one folds to bare
/// flesh. Private inner + derived [`Deref`]; build one via [`Protecting::new`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protecting(bool);

impl Protecting {
    /// Build a protecting answer from whether the piece's integrity is above zero.
    #[must_use]
    pub const fn new(protecting: bool) -> Self {
        Self(protecting)
    }
}

impl StruckPiece<'_> {
    /// Whether this worn piece still **protects** — its [`ArmorIntegrity`] is strictly
    /// above zero (`weapons-and-armor.md` §"Per-hit resolution" step 3: "useless at
    /// `≤ 0`"). The struck-vs-bare-flesh gate the fold reads (mirrors the old
    /// `WornArmor::protects`).
    #[must_use]
    pub fn protects(&self) -> Protecting {
        Protecting::new(**self.integrity > 0)
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

/// The **per-kind verdict** of one folded round — WHAT the fold did, one closed
/// variant per struck kind (GTW-573 C1).
///
/// Exactly one variant per report: the old parallel `Option` bag (whose "at most one
/// of these is `Some`" rule lived in prose) is structurally impossible here. Each
/// variant carries its kind's whole payload, owned by that kind's
/// `kinds` module (P10):
///
/// - [`Ganger`](HitVerdict::Ganger) — a hit that LANDED on a live ganger: the boxed
///   [`GangerVerdict`] (target / part / applied damage incl. the closed
///   [`ArmorWearOutcome`](crate::armor_wear::ArmorWearOutcome) / rolled injury / DOT
///   decision). Boxed: the injury payload owns a `Vec` + texts, far larger than every
///   other variant (clippy `large_enum_variant`).
/// - [`Cover`](HitVerdict::Cover) — a cover hit: the [`CoverVerdict`] (destroyed
///   `(cell, level)`, `None` when merely chipped). GTW-364.
/// - [`Slab`](HitVerdict::Slab) — a floor/roof-slab hit: the [`SlabVerdict`] (the
///   slab mirror). GTW-365.
/// - [`Ground`](HitVerdict::Ground) — a ground strike: the [`GroundAccrual`]
///   (damaged-never-destroyed cosmetic accrual). GTW-366.
/// - [`NoEffect`](HitVerdict::NoEffect) — the fold did NOTHING: a clean miss, the
///   corpse-skip, the defensive no-part / non-queryable-target ganger folds. (What
///   the round GEOMETRICALLY struck still rides [`HitReport::kind`] — a corpse-skip
///   is `kind: Ganger` + `NoEffect`, distinct from a `kind: Miss`.)
///
/// Every downstream reader is an EXHAUSTIVE match over this enum — the fire bridge
/// ([`emit_round_signals`](crate::acts::dispatch_fire)), the weapon-shove probe
/// ([`struck_ganger`](HitVerdict::struck_ganger)), and the presenter's FCT classifier
/// — so adding a struck kind is a compile error at every boundary until it is
/// bridged (GTW-573 C4 / C5).
///
/// [`Clone`] + [`PartialEq`] but NOT `Copy`/`Eq` (the ganger injury payload owns a
/// `Vec`, and its effects may carry an `f32`). Compared with `==` in seeded-replay
/// tests, never keyed in a set.
#[derive(Debug, Clone, PartialEq)]
pub enum HitVerdict {
    /// A hit LANDED on a live ganger — the boxed wound verdict.
    Ganger(Box<GangerVerdict>),
    /// A cover hit — chipped or destroyed (GTW-364).
    Cover(CoverVerdict),
    /// A floor/roof-slab hit — chipped or destroyed (GTW-365).
    Slab(SlabVerdict),
    /// A ground strike — the cosmetic damage accrual (GTW-366).
    Ground(GroundAccrual),
    /// The fold did nothing — a miss, a corpse-skip, or a defensive ganger fold.
    NoEffect,
}

impl HitVerdict {
    /// The LIVE ganger this round's fold CONNECTED with, else [`None`] — the
    /// weapon-shove probe's read (GTW-525 via GTW-573 C4).
    ///
    /// An EXHAUSTIVE match (no `_` arm): a new struck kind fails to compile here
    /// until it declares whether it counts as a connecting ganger hit. Only a
    /// [`Ganger`](HitVerdict::Ganger) verdict names a target — a corpse-skip /
    /// defensive fold is [`NoEffect`](HitVerdict::NoEffect) and shoves nobody (the
    /// fold explicitly did nothing to it).
    #[must_use]
    pub fn struck_ganger(&self) -> Option<Entity> {
        match self {
            Self::Ganger(verdict) => Some(verdict.target),
            Self::Cover(_) | Self::Slab(_) | Self::Ground(_) | Self::NoEffect => None,
        }
    }
}

/// The **frozen per-hit report** [`resolve_and_apply`](super::resolve_and_apply)
/// returns — the entire E3.9 fold's verdict (the frozen per-round report the
/// authoritative model hands the view; ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// Two fields, two questions:
///
/// - [`kind`](HitReport::kind) — what the shot GEOMETRICALLY struck (the
///   [`ShotOutcome`](crate::resolve_coarse::ShotOutcome)'s [`ShotKind`], carrying
///   the struck ganger / cover-entry / surface-cell payload the coarse pipeline
///   resolved). Echoed verbatim off the outcome — the trajectory's answer.
/// - [`verdict`](HitReport::verdict) — what the fold DID about it (the closed
///   per-kind [`HitVerdict`]). The two differ exactly where the fold declined to
///   act: a corpse-skip reads `kind: Ganger` + `NoEffect` ("the report still names
///   what the shot struck"). Both are written by the ONE
///   [`resolve_and_apply`](super::resolve_and_apply) dispatch, never assembled
///   independently.
///
/// A value object of named domain types (no bare primitive, **no pixel** — it
/// carries only damage / wound math, never a screen coordinate). The presenter reads
/// it for FX staging; [`resolve_and_apply`](super::resolve_and_apply) owns no
/// mutation after it is returned.
///
/// [`Clone`] but NOT `Copy` (the ganger verdict owns the rolled injury), and
/// [`PartialEq`] but NOT `Eq` (GTW-444: an injury effect may carry a
/// [`MovementCostMul`](crate::injuries::InjuryEffect::MovementCostMul) `f32`). A
/// report is cloned where it is forwarded (the fire path's per-round `ShotFired`
/// emission) and compared with `==` in seeded-replay tests, never keyed in a set.
#[derive(Debug, Clone, PartialEq)]
pub struct HitReport {
    /// What the shot struck — the [`ShotOutcome`](crate::resolve_coarse::ShotOutcome)'s [`ShotKind`].
    pub kind:    ShotKind,
    /// What the fold did about it — the closed per-kind [`HitVerdict`].
    pub verdict: HitVerdict,
}

impl HitReport {
    /// Build a **no-effect** report for `kind` — the fold did nothing (a clean miss,
    /// the corpse-skip, and the defensive no-part / non-queryable-target ganger
    /// folds). The `kind` still names what the shot geometrically struck.
    ///
    /// `pub` so the E4.5 `fire()` act (GTW-198) can fold a miss / defensive round to
    /// a no-effect report cross-module without re-deriving the shape. `const` (the
    /// variant is a unit; `kind` is `Copy`).
    #[must_use]
    pub const fn no_effect(kind: ShotKind) -> Self {
        Self {
            kind,
            verdict: HitVerdict::NoEffect,
        }
    }
}
