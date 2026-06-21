//! The wound-application fold + its terminal gates — the [`apply_hit`] primitive
//! and the [`GangerHitTarget`] mutable-borrow bundle it folds onto.

use bevy::prelude::Entity;

use crate::{
    armor::{ArmorIntegrity, BodyPart},
    armor_wear::{ArmorWearOutcome, wear_armor},
    ganger::{Hp, LifeState, Wounds},
    inflicted_wound::{InflictedWound, InflictedWounds},
    resolve_hit::HitResult,
    severity::Severity,
    tuning::{CombatTuning, WoundCost, WoundCosts},
};

/// The bundle of **mutable** ganger-state borrows [`apply_hit`] folds a hit onto —
/// the battle-state surfaces a hit can change.
///
/// Grouping the `&mut` borrows into one named struct keeps [`apply_hit`] under
/// clippy's argument-count gate (the same precedent as
/// [`crate::severity::SeverityInputs`] / [`crate::resolve_coarse::ShotInputs`]).
/// Every field is an existing named domain component ([`Hp`] / [`Wounds`] /
/// [`LifeState`] / [`ArmorIntegrity`] / [`InflictedWounds`] — no-bare-types),
/// exclusively borrowed so the application mutates them in place. The caller (a Bevy
/// system, or the E3.9 capstone) assembles this from the ganger entity's components
/// and — since GTW-323 (ADR-0004) — the **struck piece entity's** [`ArmorIntegrity`]
/// (resolved from `ganger → Wears → the BodyPart-tagged piece`), not a `WornArmor`
/// array slot.
pub struct GangerHitTarget<'a> {
    /// The ganger's hit-points pool — the HP loss subtracts from it (always).
    pub hp:        &'a mut Hp,
    /// The ganger's Wounds (life) pool — the severity tier spends from it.
    pub wounds:    &'a mut Wounds,
    /// The ganger's terminal life state — the gates set it (Dead trumps Downed).
    pub life:      &'a mut LifeState,
    /// The struck worn-armor **piece entity's** durability — wears in place (the
    /// piece resolved from `ganger → Wears → the BodyPart-tagged piece`; ADR-0004).
    /// `None` when the struck location wears no protecting piece (bare flesh — there
    /// is nothing to wear), so the wear step folds to [`ArmorWearOutcome::Unaffected`].
    pub integrity: Option<&'a mut ArmorIntegrity>,
    /// The ganger's inflicted-wound record (GTW-279) — each registered (non-graze)
    /// wound appends its tier + struck part here, in the SAME place the [`Wounds`]
    /// pool is spent (the additive presentation record; never read for combat math).
    pub inflicted: &'a mut InflictedWounds,
}

/// The [`Wounds`]-budget cost a non-`Fatal` [`Severity`] tier spends from the life
/// pool (`docs/combat/wounds-and-roster.md` §"Severity tiers").
///
/// [`Severity::None`] is a graze and costs **nothing** (`0`); the three middle
/// tiers read their tunable cost off `costs` ([`crate::tuning::WoundCosts`]).
/// [`Severity::Fatal`] is **not** a fixed cost — it empties the whole pool — so it
/// is handled **structurally** at the [`apply_hit`] call site (it returns the
/// floor `0` here, but `apply_hit` never reaches this branch for Fatal). Returns a
/// named [`WoundCost`] (no bare `u8`). `costs` is taken by value — it is a small
/// `Copy` tuning bundle.
#[must_use]
pub(super) const fn wound_cost(severity: Severity, costs: WoundCosts) -> WoundCost {
    match severity {
        // A graze (None) and the structurally-handled Fatal cost nothing *here*:
        // None is a true zero spend; Fatal empties the pool at the call site, so it
        // never reaches this helper (it falls through to the floor for totality).
        Severity::None | Severity::Fatal => WoundCost::new(0),
        Severity::Minor => costs.minor,
        Severity::Major => costs.major,
        Severity::Critical => costs.critical,
    }
}

/// Saturating-cast a non-negative [`HpDamage`](crate::resolve_hit::HpDamage)
/// magnitude (`i32 ≥ 0`) into the unsigned [`Hp`] inner type (`u16`), clamping into
/// the `u16` range so a wild value can never wrap or lose its sign.
///
/// The per-hit formula is signed `i32`, but HP-loss damage is `max(floor, inner)`
/// against a non-negative floor and reaches the HP pool as an unsigned count. A
/// value below `0` clamps to `0` and a value above `u16::MAX` clamps to `u16::MAX`,
/// so the subsequent [`u16::saturating_sub`] never underflows. The clamp +
/// localized `#[expect]` is the crate's guarded-cast idiom (see
/// [`crate::resolve_hit`]'s `round_to_i32` / [`crate::metric`]'s `floor_to_i32`),
/// so no `unwrap`/`expect` is needed. (Not a `const fn`: `i32::clamp` comes from
/// `Ord`, which is not yet const-callable.)
fn hp_damage_to_u16(damage: i32) -> u16 {
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "clamped into [0, u16::MAX] first, so the cast can neither wrap nor lose a sign"
    )]
    let clamped = damage.clamp(0, i32::from(u16::MAX)) as u16;
    clamped
}

/// Fold **one resolved hit** onto a ganger — HP loss + Wounds-by-severity + armor
/// wear + the terminal gates (`docs/combat/resolution.md` §9;
/// `docs/combat/wounds-and-roster.md` §"From damage to injury" / §"Severity tiers"
/// / §"Downed → death … state machine").
///
/// Takes the **already-computed** [`HitResult`] (E3.3) and rolled [`Severity`]
/// (E3.4) as inputs — it applies them, it does not recompute them (the
/// resolve → severity → apply chain is the E3.9 capstone). `part` is the struck
/// [`BodyPart`] (E3.4's location roll), `ganger` is the owning [`Entity`] (carried
/// on the returned [`ArmorBroken`](crate::armor_wear::ArmorBroken) /
/// [`ArmorWorn`](crate::armor_wear::ArmorWorn)), and `tuning` supplies the per-tier
/// [`crate::tuning::WoundCosts`].
///
/// Order (verbatim, see the module docs):
///
/// 1. **Corpse-skip** — if the ganger is already [`LifeState::Dead`], return
///    [`ArmorWearOutcome::Unaffected`] and mutate nothing.
/// 2. **HP loss — always** — subtract the [`HitResult`]'s
///    [`HpDamage`](crate::resolve_hit::HpDamage) from [`Hp`] (saturating at `0`),
///    even on a graze.
/// 3. **Wounds by tier + record** — [`Severity::Fatal`] sets [`Wounds`] to `0`
///    (empties the pool); otherwise subtract [`wound_cost`] (saturating at `0`). In
///    the SAME branch, every **non-`None`** tier (Minor / Major / Critical / Fatal —
///    a wound actually registered) appends one [`InflictedWound`] (this tier + the
///    struck `part`) to [`InflictedWounds`]; a [`Severity::None`] graze records
///    nothing (HP loss only, no Wound — `docs/combat/resolution.md` §6).
/// 4. **Armor wear** — persist the [`HitResult`]'s integrity wear onto the struck
///    piece entity's [`ArmorIntegrity`](crate::armor::ArmorIntegrity) via
///    [`wear_armor`], capturing the [`ArmorWearOutcome`]:
///    [`Broke`](ArmorWearOutcome::Broke) on the protecting→broken crossing,
///    [`Worn`](ArmorWearOutcome::Worn) on a reduction that did not break it, or
///    [`Unaffected`](ArmorWearOutcome::Unaffected).
/// 5. **Terminal gates** — `Wounds == 0` → [`LifeState::Dead`] (**trumps**); else
///    `Hp == 0` → [`LifeState::Downed`].
///
/// Returns the [`ArmorWearOutcome`] this hit produced (GTW-313) — the caller maps
/// it to the matching message(s) /
/// [`HitReport`](crate::resolve_and_apply::HitReport) fields. A corpse-skip returns
/// [`ArmorWearOutcome::Unaffected`]. Pure, render-free, saturating arithmetic — no
/// underflow, no pixel.
#[must_use]
pub fn apply_hit(
    target: GangerHitTarget<'_>,
    hit: &HitResult,
    severity: Severity,
    part: BodyPart,
    ganger: Entity,
    tuning: &CombatTuning,
) -> ArmorWearOutcome {
    // (a) Corpse-skip: a dead ganger is final — mutate nothing, emit nothing.
    if *target.life == LifeState::Dead {
        return ArmorWearOutcome::Unaffected;
    }

    // (b) HP loss ALWAYS — saturating at 0 (Hp is unsigned; a lethal hit depletes
    // the pool to 0, never underflows). Applies even on a Severity::None graze.
    let hp_loss = hp_damage_to_u16(*hit.hp_damage);
    *target.hp = Hp::new(target.hp.saturating_sub(hp_loss));

    // (c) Wounds by severity tier. Fatal EMPTIES the pool (structural); every other
    // tier spends its tunable cost, saturating at 0.
    if severity == Severity::Fatal {
        *target.wounds = Wounds::new(0);
    } else {
        let cost = *wound_cost(severity, tuning.wound_costs);
        *target.wounds = Wounds::new(target.wounds.saturating_sub(cost));
    }

    // (c2) Record the inflicted wound (GTW-279) — every NON-None tier registered a
    // wound on the pool above, so it appends one record (this tier + the struck
    // part) in infliction order. A Severity::None graze spent no pool and records
    // nothing (HP loss only — resolution.md §6). The list is additive and never read
    // back for combat math (a presentation record for the GTW-278 panels).
    if severity != Severity::None {
        target.inflicted.record(InflictedWound::new(severity, part));
    }

    // (d) Armor wear — persist this hit's integrity wear onto the struck worn piece
    // ENTITY's integrity component (E3.5; the piece resolved from `ganger → Wears`,
    // ADR-0004 / GTW-323), capturing the per-hit ArmorWearOutcome (Broke crossing /
    // Worn reduction / Unaffected). A struck location with no protecting piece (bare
    // flesh — `integrity == None`) wears nothing, folding to Unaffected. The wear
    // mutation itself is byte-identical to the pre-GTW-323 array-slot wear.
    let wear_outcome = match target.integrity {
        Some(integrity) => wear_armor(integrity, part, hit.wear, ganger),
        None => ArmorWearOutcome::Unaffected,
    };

    // (e) Terminal gates, in order — Wounds depleted to 0 → Dead (TRUMPS Downed,
    // checked first); else Hp depleted to 0 → Downed. (Pools are unsigned, so the
    // doc's "≤ 0" gate is "== 0 after the saturating spend".)
    if *target.wounds == Wounds::new(0) {
        *target.life = LifeState::Dead;
    } else if *target.hp == Hp::new(0) {
        *target.life = LifeState::Downed;
    }

    wear_outcome
}
