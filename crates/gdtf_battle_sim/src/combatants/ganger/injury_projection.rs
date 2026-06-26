//! [`derive_stats_with_injuries`] — the GTW-436 **injury-aware** projection: the
//! GTW-405 modifier layer applied on top of the pure GTW-384
//! [`derive_stats`](crate::ganger::derive_stats), plus the gate-enforced
//! [`effective_toughness`] / [`effective_luck`] single read paths for a direct
//! attribute.
//!
//! **The modifier-layer invariant** (`docs/combat/resolution.md` injury tables;
//! GTW-405): base attributes stay AUTHORITATIVE and are never mutated by an injury;
//! the per-ganger [`InflictedInjuries`] ledger is the SOLE source of every injury
//! stat delta; this projector RE-SUMS those deltas on every projection (never
//! applied-once), so every stored derived stat is `f(BaseAttributes,
//! GangerStatTuning, InflictedInjuries)` and a `stat_tuning.ron` hot-reload
//! re-applies the deltas by construction rather than wiping them.
//!
//! The three delta layers, in order (`docs/combat/resolution.md`; the GTW-405 design):
//!
//! 1. **Attribute layer (PRE-derivation).** `effective_attr[a] = base[a] + Σ
//!    Modify(a)` is folded into the eight attributes FIRST, then fed into
//!    [`derive_stats`] — so a `Modify(Aim, -2)` RIPPLES through every derived stat
//!    that reads Aim (the derived [`Shooting`](crate::ganger::Shooting) drops
//!    automatically).
//! 2. **Skill layer (POST-derivation).** `skill[s] = derived[s] + Σ Modify(s)` adds
//!    on top of the four derived skill stats
//!    ([`Shooting`](crate::ganger::Shooting) / [`Fight`](crate::ganger::Fight) /
//!    [`Reactions`](crate::ganger::Reactions) / [`Morale`](crate::ganger::Morale)),
//!    independent of the attribute layer.
//! 3. **Pool layer (POST-derivation).** `max[p] = derived_max[p] + Σ Modify(p)` docks
//!    the four derived MAXES ([`TuMax`](crate::ganger::TuMax) /
//!    [`HpMax`](crate::ganger::HpMax) / [`WoundsMax`](crate::ganger::WoundsMax) /
//!    Bottle); the current pools are CLAMPED via `min(current, new_max)` by the
//!    caller. A pool dock FLOORS the new max at `1` (never `0`), so a `Modify` can
//!    never itself reduce a live current pool to `0` — only the injury table's
//!    `Fatal` severity kills (`docs/combat/resolution.md` §6).

use crate::{
    ganger::{
        Aim, Bottle, Cool, DerivedStats, Fight, GangerAttributes, Grit, Hp, HpMax, Luck, Morale,
        Reactions, Reflexes, Shooting, Speed, Strength, Toughness, Tu, TuMax, Wounds, WoundsMax,
        derive_stats,
    },
    injuries::{InflictedInjuries, StatDeltaSum, StatTarget},
    tuning::GangerStatTuning,
};

/// The effective value of a direct **attribute** — the base attribute value plus the
/// ledger's summed `Modify` delta for it (`docs/combat/resolution.md` injury tables;
/// GTW-405) — as a bare `f32`.
///
/// The shared core of [`effective_toughness`] / [`effective_luck`]'s typed wrappers and
/// the [`effective_attributes`] PRE-derivation fold: the inner is added as `f32` (the
/// delta sum is an `i16`, the attributes are `f32`), so a positive (boost) or
/// negative (debuff) delta shifts the value either way. A debuff that drives an
/// attribute below zero is left as the raw (possibly negative) effective value — the
/// derivation's pool rounding already clamps the final pools at `0`.
fn effective_attr_value(base: f32, sum: StatDeltaSum) -> f32 {
    base + f32::from(*sum)
}

/// The gate-enforced **single read path** for a direct [`Toughness`] — the base value
/// plus the ledger's summed `Modify(Toughness)` delta (`docs/combat/resolution.md`
/// injury tables; GTW-405).
///
/// Every direct read of [`Toughness`] (today the §6 severity roll's mitigation term)
/// MUST go through this accessor rather than the raw component, so a
/// `Modify(Toughness)` injury — which the derived stats already see through
/// [`derive_stats_with_injuries`] — also shifts the severity roll, closing the
/// divergence the modifier layer would otherwise open.
#[must_use]
pub fn effective_toughness(base: Toughness, ledger: &InflictedInjuries) -> Toughness {
    Toughness::new(effective_attr_value(
        *base,
        ledger.delta_for(StatTarget::Toughness),
    ))
}

/// The gate-enforced **single read path** for a direct [`Luck`] — the base value plus
/// the ledger's summed `Modify(Luck)` delta (`docs/combat/resolution.md` injury
/// tables; GTW-405).
///
/// The [`Luck`] companion to [`effective_toughness`]: every direct read of [`Luck`]
/// (today both gangers' Luck in the §6 severity roll) MUST go through this accessor,
/// so a `Modify(Luck)` injury shifts the severity roll's directional tail in step
/// with the derived stats.
#[must_use]
pub fn effective_luck(base: Luck, ledger: &InflictedInjuries) -> Luck {
    Luck::new(effective_attr_value(
        *base,
        ledger.delta_for(StatTarget::Luck),
    ))
}

/// Fold the ledger's eight attribute-`Modify` deltas onto the base attributes — the
/// PRE-derivation effective-attribute layer (`docs/combat/resolution.md` injury
/// tables; GTW-405).
///
/// Returns a fresh [`GangerAttributes`] whose every field is `base + Σ Modify(field)`,
/// so feeding it into [`derive_stats`] ripples each attribute delta through every
/// derived stat that reads it. Pure — the base attributes are never mutated. An
/// empty / absent ledger yields the base attributes unchanged (the zero-delta
/// identity).
fn effective_attributes(base: &GangerAttributes, ledger: &InflictedInjuries) -> GangerAttributes {
    GangerAttributes {
        speed:     Speed::new(effective_attr_value(
            *base.speed,
            ledger.delta_for(StatTarget::Speed),
        )),
        aim:       Aim::new(effective_attr_value(
            *base.aim,
            ledger.delta_for(StatTarget::Aim),
        )),
        strength:  Strength::new(effective_attr_value(
            *base.strength,
            ledger.delta_for(StatTarget::Strength),
        )),
        toughness: effective_toughness(base.toughness, ledger),
        reflexes:  Reflexes::new(effective_attr_value(
            *base.reflexes,
            ledger.delta_for(StatTarget::Reflexes),
        )),
        cool:      Cool::new(effective_attr_value(
            *base.cool,
            ledger.delta_for(StatTarget::Cool),
        )),
        grit:      Grit::new(effective_attr_value(
            *base.grit,
            ledger.delta_for(StatTarget::Grit),
        )),
        luck:      effective_luck(base.luck, ledger),
    }
}

/// Add a derived-`Modify` `f32` delta onto a freshly derived skill value (the POST-
/// derivation skill layer). The delta sum is an `i16`; a skill stat is an `f32`.
fn skill_with_delta(derived: f32, sum: StatDeltaSum) -> f32 {
    derived + f32::from(*sum)
}

/// Dock a derived `u8` pool MAX by the ledger's summed `Modify` delta, with the
/// no-self-kill floor (the POST-derivation pool layer; `docs/combat/resolution.md` §6).
///
/// The new max is `derived_max + delta`, saturated into `u8`. The lower bound is
/// `min(derived_max, 1)`: when the base derived max is already `≥ 1`, a pool `Modify`
/// can never drive the ceiling below `1` (and thus, via the caller's `min(current,
/// new_max)` clamp, can never itself reduce a LIVE current pool to `0` / self-kill — only
/// the injury table's `Fatal` severity kills). When the base derived max is `0` (a
/// degenerate base derivation, e.g. zeroed tuning weights — NOT an injury), the lower
/// bound is `0`, so the ZERO-DELTA IDENTITY is preserved exactly (an empty ledger derives
/// precisely as [`derive_stats`](crate::ganger::derive_stats)).
fn pool_max_u8_with_delta(derived_max: u8, sum: StatDeltaSum) -> u8 {
    let docked = i32::from(derived_max) + i32::from(*sum);
    let floor = i32::from(derived_max).min(1);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to floor..=u8::MAX above (floor is 0 or 1), so the cast cannot \
                  wrap or go negative"
    )]
    let floored = docked.clamp(floor, i32::from(u8::MAX)) as u8;
    floored
}

/// Dock a derived `u16` pool MAX by the ledger's summed `Modify` delta, with the
/// no-self-kill floor — the `u16` ([`HpMax`](crate::ganger::HpMax)) companion to
/// [`pool_max_u8_with_delta`] (same `min(derived_max, 1)` lower bound, identity-preserving
/// at a base max of `0`).
fn pool_max_u16_with_delta(derived_max: u16, sum: StatDeltaSum) -> u16 {
    let docked = i32::from(derived_max) + i32::from(*sum);
    let floor = i32::from(derived_max).min(1);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to floor..=u16::MAX above (floor is 0 or 1), so the cast cannot \
                  wrap or go negative"
    )]
    let floored = docked.clamp(floor, i32::from(u16::MAX)) as u16;
    floored
}

/// **Derive** the computed combat stats with the GTW-405 injury modifier layer applied
/// — the GTW-436 injury-aware projection on top of the pure GTW-384
/// [`derive_stats`](crate::ganger::derive_stats).
///
/// Applies the three delta layers, in order (see the module docs): (1) the eight
/// attribute deltas fold into the base attributes PRE-derivation (so they ripple
/// through every derived stat), then [`derive_stats`] runs on the EFFECTIVE
/// attributes; (2) the four skill deltas add onto the derived skills POST-derivation;
/// (3) the four pool deltas dock the derived MAXES POST-derivation (floored at `1`).
///
/// The returned [`DerivedStats`]'s `*_max` fields carry the docked ceilings; its
/// current-pool fields ([`Tu`](crate::ganger::Tu) / [`Hp`](crate::ganger::Hp) /
/// [`Wounds`](crate::ganger::Wounds)) are set EQUAL to those docked maxes here (the
/// "full at battle start" derivation contract) — the live re-derive path (the
/// `rederive_one` projection helper) is what CLAMPS a damaged current pool via
/// `min(current, new_max)`, never resetting it.
///
/// An empty / absent ledger ([`InflictedInjuries::default`]) yields EXACTLY
/// [`derive_stats`]'s output (the zero-delta identity), so a ganger with no injuries
/// derives precisely as before. Pure — no RNG, no global state, the base attributes
/// are never mutated.
#[must_use]
pub fn derive_stats_with_injuries(
    base: &GangerAttributes,
    tuning: &GangerStatTuning,
    ledger: &InflictedInjuries,
) -> DerivedStats {
    // (1) PRE-derivation: fold the attribute deltas in, then derive on the effective
    // attributes so each attribute shift ripples through every derived stat.
    let effective = effective_attributes(base, ledger);
    let derived = derive_stats(&effective, tuning);

    // (2) POST-derivation skill layer: add the derived-Modify deltas onto the four
    // skill stats, independent of the attribute layer.
    let shooting = Shooting::new(skill_with_delta(
        *derived.shooting,
        ledger.delta_for(StatTarget::Shooting),
    ));
    let fight = Fight::new(skill_with_delta(
        *derived.fight,
        ledger.delta_for(StatTarget::Fight),
    ));
    let reactions = Reactions::new(skill_with_delta(
        *derived.reactions,
        ledger.delta_for(StatTarget::Reactions),
    ));
    let morale = Morale::new(skill_with_delta(
        *derived.morale,
        ledger.delta_for(StatTarget::Morale),
    ));

    // (3) POST-derivation pool layer: dock the derived MAXES (floored at 1); the
    // current pools track the docked max here (the caller clamps a damaged pool).
    let tu_max = pool_max_u8_with_delta(*derived.tu_max, ledger.delta_for(StatTarget::Tu));
    let hp_max = pool_max_u16_with_delta(*derived.hp_max, ledger.delta_for(StatTarget::Hp));
    let wounds_max =
        pool_max_u8_with_delta(*derived.wounds_max, ledger.delta_for(StatTarget::Wounds));
    let bottle = pool_max_u8_with_delta(*derived.bottle, ledger.delta_for(StatTarget::Bottle));

    DerivedStats {
        shooting,
        fight,
        reactions,
        morale,
        tu: Tu::new(tu_max),
        tu_max: TuMax::new(tu_max),
        hp: Hp::new(hp_max),
        hp_max: HpMax::new(hp_max),
        wounds: Wounds::new(wounds_max),
        wounds_max: WoundsMax::new(wounds_max),
        bottle: Bottle::new(bottle),
    }
}
