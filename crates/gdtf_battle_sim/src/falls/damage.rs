//! The pure **fall-damage synthesis** (GTW-523 C4 / C5) — [`resolve_fall_hit`], the
//! weight-free fall-damage fork that routes a fall's blow through the SHARED
//! wound-synthesis core the ganger path also uses (no new formula, no new RNG stream).
//!
//! A slab-destroy fall has **no attacker** — no shooter, no weapon entity, no opposed roll.
//! So the fork synthesizes the minimal inputs the shared core needs:
//!
//! - **damage** = [`PerStoreyDamage`](crate::tuning::PerStoreyDamage) × storeys fallen
//!   (LINEAR, weight-free — GTW-452 owns weighting), as a synthetic
//!   [`WeaponDamage`](crate::weapon::WeaponDamage);
//! - **punch / shred** = `0` (a fall neither penetrates nor shreds armor beyond the soak);
//! - **matchup** = [`Matchup::Neutral`](crate::matchup::Matchup::Neutral) (kinetic vs no
//!   armor-type wheel node — armor's `protection` still soaks, only the wheel advantage is
//!   absent);
//! - **`fatal_bias`** = `0` (no weapon pushing the severity table);
//! - **`shooter_luck`** = `0` ([`Luck::new(0)`](crate::ganger::Luck) — there is no attacker, so
//!   the §6 shooter-Luck nasty-wound term is nil; the DEFENDER's Luck floor-extend still
//!   applies, read off the faller).
//!
//! It then hands those inputs to the SHARED
//! [`synthesize_wound`](crate::resolve_and_apply::synthesize_wound) core (GTW-523
//! remediation) — the SAME `resolve_hit` → `roll_severity` (one
//! [`SeverityRng`](crate::rng::SeverityRng) draw) → `apply_hit` → `roll_injury` (one
//! [`InjuryRng`](crate::rng::InjuryRng) draw on a non-graze/non-fatal wound) fold the
//! ganger path (`fold_ganger`) also routes through — so the §5 → §6 → §8 wound synthesis
//! lives in exactly ONE place and the two paths cannot drift. This fork reimplements NONE
//! of it; it only synthesizes the no-attacker inputs. NO `FightRng` draw (there is no
//! opposed roll).

use bevy::prelude::Entity;

use crate::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType},
    falls::StoreysFallen,
    ganger::Luck,
    injuries::{InjuryRegistry, InjuryTables, RolledInjury},
    matchup::Matchup,
    resolve_and_apply::{TargetGanger, WoundBlow, WoundCoreInputs, synthesize_wound},
    rng::{InjuryRng, SeverityRng},
    tuning::{CombatTuning, PerStoreyDamage},
    weapon::{FatalBias, WeaponDamage, WeaponPunch, WeaponShred},
};

/// The **bare-flesh** armor piece — a zeroed soak for a struck location that wears no
/// protecting piece (the [`crate::resolve_and_apply`] / [`crate::melee`] `BARE_FLESH` shape,
/// mirrored here so the falls fork stays self-contained and never reaches a private item).
///
/// Floor / protection / hardness / integrity are all `0` so the per-hit formula soaks
/// nothing (`dmg == damage`), and the type is [`ArmorType::DEFAULT`] (the matchup is forced
/// [`Matchup::Neutral`] on a fall, so the type is never matched against anyway).
const BARE_FLESH: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

/// The **weight-free fall damage** in raw HP terms — `per_storey_damage × storeys`, LINEAR
/// (GTW-523 C4). Saturating so a pathological storey count cannot overflow the `i32` the
/// per-hit formula consumes.
fn fall_damage_magnitude(per_storey: PerStoreyDamage, storeys: StoreysFallen) -> WeaponDamage {
    let magnitude = (*per_storey).saturating_mul(i32::from(*storeys));
    WeaponDamage::new(magnitude)
}

/// The **fall impact** a [`resolve_fall_hit`] call resolves — the weight-free magnitude
/// inputs (`per_storey × storeys`), the struck [`BodyPart`](crate::armor::BodyPart), and the
/// faller ([`TargetGanger`] bundle) + its entity.
///
/// A named bundle (no bare types, no long argument list) grouping the fall-specific inputs
/// with the faller borrow-view, so [`resolve_fall_hit`] stays under clippy's argument-count
/// gate with NO suppression (the [`WoundCoreInputs`] / melee `Combatants` grouping
/// precedent). Lifetime `'a` ties the faller borrows to the caller's frame.
pub(crate) struct FallImpact<'a> {
    /// The per-storey damage leaf — one factor of the weight-free `per_storey × storeys`.
    pub per_storey:    PerStoreyDamage,
    /// The storeys fallen — the other factor of the weight-free magnitude.
    pub storeys:       StoreysFallen,
    /// The struck part — [`BodyPart::Torso`](crate::armor::BodyPart::Torso) for a fall (the
    /// body lands as a whole; deterministic, no body-part draw).
    pub part:          crate::armor::BodyPart,
    /// The faller's mutable battle surfaces + read attribute stats.
    pub target:        TargetGanger<'a>,
    /// The faller's entity — passed through the shared core to `apply_hit`'s wear address.
    pub target_entity: Entity,
}

/// The **shared combat + injury environment** a [`resolve_fall_hit`] call reads — the
/// [`CombatTuning`], the injury [`InjuryTables`] / [`InjuryRegistry`] content, and the two
/// seeded draw streams the §6 severity / §8 injury roll advance.
///
/// A named bundle grouping the world content + streams the shared wound core needs, so
/// [`resolve_fall_hit`] stays under the argument-count gate with NO suppression. The two
/// streams are distinct types ([`SeverityRng`] / [`InjuryRng`]) and both `&mut` (advanced by
/// the core). REUSES the ranged pipeline's streams verbatim — no new stream, no `FightRng`
/// (a fall has no attacker — GTW-523 C7). Lifetime `'a` ties the reads / stream borrows to
/// the caller's frame.
pub(crate) struct FallWoundEnv<'a> {
    /// The shared combat tuning — the §6 severity scaling + wound costs + the §5 formula.
    pub tuning:       &'a CombatTuning,
    /// The shared weighted `(part, severity)` injury tables — the §8 roll's pool (AS-IS).
    pub tables:       &'a InjuryTables,
    /// The injury registry — resolves the §8 roll's picked key to its authored def.
    pub registry:     &'a InjuryRegistry,
    /// The §6 severity-roll stream — one draw per live faller.
    pub severity_rng: &'a mut SeverityRng,
    /// The §8 injury-roll stream — one draw per live faller with a non-graze / non-fatal wound.
    pub injury_rng:   &'a mut InjuryRng,
}

/// Resolve one fall's blow onto the faller and roll its injury — the pure §Falls damage +
/// injury synthesis (GTW-523 C4 / C5), returning the rolled [`RolledInjury`] (if any) for
/// the caller to bridge into the EXISTING
/// [`InjuryInflicted`](crate::acts::InjuryInflicted) message.
///
/// Mirrors the ganger path (`fold_ganger`, `docs/combat/resolution.md` §5 / §6) by handing
/// its synthetic no-attacker inputs to the SAME shared
/// [`synthesize_wound`](crate::resolve_and_apply::synthesize_wound) core (see the module
/// docs). It resolves ONLY the per-path inputs; the core runs the shared fold:
///
/// 1. **Damage magnitude** — `per_storey_damage × storeys` (LINEAR, weight-free).
/// 2. **Armor / bare flesh** — the struck part's worn piece (if it still protects) or the
///    zeroed [`BARE_FLESH`] piece; matchup forced
///    [`Matchup::Neutral`](crate::matchup::Matchup::Neutral) (a fall has no weapon wheel
///    node — armor `protection` still soaks, only the wheel advantage is nil).
/// 3. **Shared core** — the resolved blow is handed to
///    [`synthesize_wound`](crate::resolve_and_apply::synthesize_wound), which runs the
///    corpse-skip → `resolve_hit` (§5, punch 0 / shred 0 synthetic weapon, so armor is
///    honored) → `roll_severity` (§6, the ONE [`SeverityRng`](crate::rng::SeverityRng)
///    draw, `shooter_luck = 0` / `fatal_bias = 0` — no attacker — and the faller's own
///    Luck as the defender floor-extend) → `apply_hit` (§6) → `roll_injury` (§8, the ONE
///    [`InjuryRng`](crate::rng::InjuryRng) draw, gated on the rolled severity exactly as
///    the ganger path: a graze / fatal takes NO draw, a Minor/Major/Critical ALWAYS takes
///    EXACTLY ONE — content-independent stream alignment).
///
/// A faller already [`LifeState::Dead`](crate::ganger::LifeState::Dead) is corpse-skipped
/// BEFORE any draw (no severity, no injury draw, no mutation) — INSIDE the shared
/// [`synthesize_wound`](crate::resolve_and_apply::synthesize_wound) core, the same
/// corpse-skip the ganger path takes, so a corpse never perturbs a stream. NO `FightRng`
/// draw (a fall has no opposed roll — GTW-523 C7). Returns the rolled `Option<RolledInjury>`
/// (the faller's HP / Wounds / `LifeState` are mutated IN PLACE through `target`); pure given
/// the two injected stream cursors.
///
/// GTW-523 remediation — the `resolve_hit` → `roll_severity` → `apply_hit` → `roll_injury`
/// orchestration is NO LONGER re-run here: this fork only synthesizes the no-attacker INPUTS
/// (the `per_storey × storeys` magnitude, punch/shred `0`, the worn-piece-or-bare-flesh
/// `ArmorPiece` under [`Matchup::Neutral`](crate::matchup::Matchup::Neutral), `fatal_bias`
/// `0`, `shooter_luck` `0`) and calls the SHARED
/// [`synthesize_wound`](crate::resolve_and_apply::synthesize_wound) core the ganger path
/// also calls — so the §5 → §6 → §8 wound synthesis lives in exactly ONE place and the fall
/// / weapon paths cannot drift. The per-hit `ArmorWearOutcome` on the core's returned
/// `WoundSynthesis` verdict is not surfaced (the presenter reads the resulting `Changed<…>`
/// + the existing wound/injury signals); only the rolled injury is returned.
///
/// Takes the fall-specific [`FallImpact`] (magnitude inputs + faller) and the shared
/// [`FallWoundEnv`] (tuning + injury content + the two draw streams) as TWO named bundles —
/// so the signature passes clippy's argument-count gate with NO `#[expect]` suppression
/// (the GTW-523 remediation requirement).
pub(crate) fn resolve_fall_hit(
    impact: FallImpact<'_>,
    env: FallWoundEnv<'_>,
) -> Option<RolledInjury> {
    let FallImpact {
        per_storey,
        storeys,
        part,
        target,
        target_entity,
    } = impact;
    let FallWoundEnv {
        tuning,
        tables,
        registry,
        severity_rng,
        injury_rng,
    } = env;

    // (1) The weight-free LINEAR magnitude: per_storey_damage × storeys.
    let damage = fall_damage_magnitude(per_storey, storeys);

    // (2) Armor / bare flesh. A fall has no weapon wheel node → Matchup::Neutral regardless
    // of the struck piece's ArmorType (armor `protection` still soaks — only the wheel
    // advantage is absent). Read the struck piece's stats if it still protects, else the
    // zeroed BARE_FLESH piece.
    let piece = match target.piece.as_ref() {
        Some(p) if *p.protects() => ArmorPiece::new(
            p.floor,
            p.protection,
            p.integrity_value(),
            p.hardness,
            p.armor_type,
        ),
        _ => BARE_FLESH,
    };

    // (3) The SHARED wound-synthesis core (GTW-523 remediation): corpse-skip → resolve_hit
    // (synthetic weapon: punch 0 / shred 0, Neutral matchup) → roll_severity (the ONE
    // SeverityRng draw, shooter_luck 0 / fatal_bias 0 — no attacker) → apply_hit → roll_injury
    // (the ONE InjuryRng draw, gated on severity). Reuses the SHARED (part, severity) injury
    // pool AS-IS — NO new source dimension (GTW-452 owns falling weighting). Returns None on
    // the corpse-skip (a dead faller — no draw, no mutation), in which case no injury rolled.
    synthesize_wound(WoundCoreInputs {
        blow: WoundBlow {
            part,
            damage,
            punch: WeaponPunch::new(0),
            shred: WeaponShred::new(0),
            piece,
            matchup: Matchup::Neutral,
            fatal_bias: FatalBias::new(0.0),
            shooter_luck: Luck::new(0.0),
        },
        target,
        target_entity,
        tuning,
        severity_rng,
        tables,
        registry,
        injury_rng,
    })
    .and_then(|synthesis| synthesis.injury)
}
