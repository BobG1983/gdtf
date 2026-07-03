//! The **ganger** struck-kind — the wound arm of the E3.9 fold
//! (`docs/combat/resolution.md` §5 / §6 / §8) and its per-kind verdict payload
//! ([`GangerVerdict`] / [`AppliedDamage`]), owned here per the GTW-573 one-module-per-kind
//! layout (a new struck kind is one sibling module + one
//! [`HitVerdict`](super::super::report::HitVerdict) variant + one delegation arm).

use bevy::prelude::Entity;

use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType, BodyPart,
    },
    armor_wear::ArmorWearOutcome,
    ganger::{LifeState, Luck},
    injuries::{InjuryRegistry, InjuryTables, RolledInjury},
    matchup::{Matchup, matchup},
    resolve_and_apply::{
        report::{HitVerdict, StruckPiece, TargetGanger},
        wound_core::{WoundBlow, WoundCoreInputs, synthesize_wound},
    },
    resolve_coarse::ShotOutcome,
    resolve_hit::HitResult,
    rng::{InjuryRng, SeverityRng},
    severity::Severity,
    tuning::CombatTuning,
    weapon::{Dot, WeaponStats},
};

/// The **bare-flesh** armor piece — a zeroed soak used when the struck location no
/// longer protects (its piece entity's [`ArmorIntegrity`](crate::armor::ArmorIntegrity)
/// `≤ 0`; `weapons-and-armor.md` §"Per-hit resolution": "later hits on that location
/// resolve as bare flesh").
///
/// Floor / protection / hardness are all `0` so the per-hit formula soaks nothing
/// (on bare flesh `dmg == damage` regardless of matchup, since protection `0`),
/// integrity is `0` (the piece is already worn through — there is nothing to wear),
/// and the type is [`ArmorType::DEFAULT`] (an explicit placeholder — the matchup on
/// bare flesh is forced to [`Matchup::Neutral`], so the armor type is never matched
/// against). A `const` since every stat is a constant zero.
const BARE_FLESH: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

/// The **applied-damage block** of a landed ganger hit — the resolved damage of a hit
/// that connected with a live ganger (`docs/combat/resolution.md` §5 / §6).
///
/// A frozen `Copy` record of named newtypes (no bare primitive, no pixel): the
/// resolved [`Matchup`], the per-hit [`HitResult`], the rolled [`Severity`], the
/// ganger's [`LifeState`] **after** application, and the per-hit armor-wear
/// [`ArmorWearOutcome`] — the CLOSED broke / worn / unaffected verdict `apply_hit`
/// produced, carried directly (GTW-573 un-flattened it from the old prose-exclusive
/// `broken` / `worn` `Option` pair, so a both-`Some` state is unrepresentable). The
/// presenter reads it for FX; it is never mutated after
/// [`resolve_and_apply`](super::super::resolve_and_apply) returns. Lives inside a
/// [`GangerVerdict`] — a hit that did NOT land on a live ganger has no applied block at
/// all (its verdict is a different [`HitVerdict`] variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedDamage {
    /// The resolved weapon×armor matchup (E3.2) — [`Matchup::Neutral`] on bare flesh.
    pub matchup:    Matchup,
    /// The resolved per-hit damage / penetration / wear (E3.3).
    pub hit:        HitResult,
    /// The rolled wound severity bucket (E3.4) — the ONE severity draw's outcome.
    pub severity:   Severity,
    /// The target's [`LifeState`] **after** the hit was applied (E3.6's terminal gates).
    pub life_after: LifeState,
    /// The per-hit armor-wear outcome (E3.6 / GTW-313): [`Broke`](ArmorWearOutcome::Broke)
    /// on the protecting→broken crossing, [`Worn`](ArmorWearOutcome::Worn) on a reduction
    /// short of breaking (carrying the integrity delta), [`Unaffected`](ArmorWearOutcome::Unaffected)
    /// on a bare-flesh / zero-wear hit. One closed enum — the mutual exclusivity is
    /// structural, not prose.
    pub wear:       ArmorWearOutcome,
}

/// The **ganger verdict** — everything one round that LANDED on a live ganger froze:
/// the struck target, the struck [`BodyPart`], the [`AppliedDamage`] block, the rolled
/// injury, and the DOT-attach decision (GTW-573 C1's per-kind payload).
///
/// Carried BOXED in [`HitVerdict::Ganger`] — the [`injury`](GangerVerdict::injury)
/// owns a [`RolledInjury`] (a `Vec` of effects + three texts), much larger than the
/// other kinds' payloads, so boxing keeps the verdict enum small
/// (clippy `large_enum_variant`). A corpse-skip / defensive no-part / non-queryable
/// target never builds one — those fold to [`HitVerdict::NoEffect`], so a
/// `GangerVerdict` ALWAYS means the hit applied (no `Option` fields for the blow
/// itself — illegal states unrepresentable).
#[derive(Debug, Clone, PartialEq)]
pub struct GangerVerdict {
    /// The struck ganger entity — the address the fire bridge emits the per-round
    /// injury / DOT / on-death messages to. A Bevy [`Entity`] handle (framework
    /// plumbing, the one bare type the no-bare-types rule permits in a payload).
    pub target:      Entity,
    /// The struck [`BodyPart`] (the §4 location roll, drawn upstream).
    pub part:        BodyPart,
    /// The applied-damage block — matchup / hit / severity / life-after / wear.
    pub applied:     AppliedDamage,
    /// The injury this round rolled (GTW-438) — `Some(`[`RolledInjury`]`)` ONLY on a
    /// non-graze, non-fatal [`Severity`] whose `(part, severity)` table rolled a named
    /// injury; the fire path bridges it to an
    /// [`InjuryInflicted`](crate::acts::InjuryInflicted) message. `None` on a graze /
    /// `Fatal`, or an empty/missing table (the roll still took its one
    /// [`InjuryRng`](crate::rng::InjuryRng) draw, then discarded it — the
    /// content-independent stream-alignment property).
    pub injury:      Option<RolledInjury>,
    /// The DOT this round attached (GTW-544) — `Some(`[`Dot`]`)` ONLY when the firing
    /// weapon carries a [`DotProfile`](crate::weapon::DotProfile) AND the hit PENETRATED
    /// armor ([`PenetratingDamage`](crate::resolve_hit::PenetratingDamage) `> 0`); the
    /// fire path bridges it to a [`DotApplied`](crate::acts_runtime::dot::DotApplied)
    /// message the [`apply_dot`](crate::acts_runtime::dot::apply_dot) boundary attaches
    /// (or REFRESHES — DOTs do not stack). `None` for a fully-soaked hit or a non-DOT
    /// weapon. A frozen decision, not a live mutation (the fold owns no component
    /// attach; the boundary system does), taking no RNG draw.
    pub dot_applied: Option<Dot>,
}

/// Resolve the struck location to the `(`[`ArmorPiece`]`, `[`Matchup`]`)` the
/// per-hit formula runs against — the armored branch or **bare flesh**.
///
/// If a worn [`StruckPiece`] is present AND still [`protects`](StruckPiece::protects),
/// returns that piece's stats (read off its piece-entity components — GTW-323 /
/// ADR-0004) and the E3.2 [`matchup`] of the weapon's
/// [`DamageType`](crate::weapon::DamageType) vs the piece's [`ArmorType`] (the wheel
/// advantage applies). Otherwise (no piece at this location, or it is worn through)
/// returns the zeroed [`BARE_FLESH`] piece under [`Matchup::Neutral`] — there is no
/// armor type to match against, so no wheel advantage, and the zeroed soak means the
/// hit lands as full weapon damage (`weapons-and-armor.md` §"Per-hit resolution").
fn struck_piece(piece: Option<&StruckPiece<'_>>, weapon: WeaponStats<'_>) -> (ArmorPiece, Matchup) {
    match piece {
        Some(p) if p.protects() => {
            // Re-assemble the read-only ArmorPiece value the damage formula consumes
            // from the piece entity's stat components (integrity is read by value here;
            // the wear mutation happens later, inside the shared core's `apply_hit`).
            let assembled = ArmorPiece::new(
                p.floor,
                p.protection,
                p.integrity_value(),
                p.hardness,
                p.armor_type,
            );
            let resolved = matchup(*weapon.damage_type, p.armor_type);
            (assembled, resolved)
        }
        // No protecting piece (missing piece OR worn through) ⇒ bare flesh: no
        // protection / hardness, no armor type to match → Neutral.
        _ => (BARE_FLESH, Matchup::Neutral),
    }
}

/// Fold a **[`ShotKind::Ganger`](crate::resolve_coarse::ShotKind::Ganger)** outcome
/// onto the target ganger — the wound arm of the E3.9 fold
/// (`docs/combat/resolution.md` §5 / §6 / §8), returning the per-kind
/// [`HitVerdict`].
///
/// The wound path's own draw discipline (the load-bearing seeded-replay contract —
/// pinned by `test::draw_discipline`):
///
/// - a `None` `target` (the struck entity is not a queryable ganger) or a defensive
///   `None` `body_part` folds to [`HitVerdict::NoEffect`] with **no draw on either
///   stream**;
/// - a corpse (target already [`LifeState::Dead`]) short-circuits inside
///   [`synthesize_wound`] BEFORE any draw — [`HitVerdict::NoEffect`], **neither** draw;
/// - a LIVE hit takes EXACTLY ONE [`SeverityRng`] draw, and the [`InjuryRng`] draw is
///   gated on the rolled [`Severity`]: `Minor`/`Major`/`Critical` take EXACTLY ONE
///   (even on an empty/missing table — draw-then-discard, content-independent stream
///   alignment); a graze ([`Severity::None`]) / `Fatal` takes NONE.
///
/// Composes the shared verbs — [`struck_piece`] (armored piece + matchup, or bare
/// flesh), then the GTW-523 [`synthesize_wound`] core (corpse-skip → `resolve_hit` →
/// `roll_severity` → `apply_hit` → `roll_injury`) — and freezes the core's verdict
/// into the boxed [`GangerVerdict`], carrying the [`ArmorWearOutcome`] DIRECTLY
/// (GTW-573 C2 — no broken/worn Option flattening). The GTW-544 DOT-attach decision
/// (the weapon carries a `DotProfile` AND the pre-floor penetrating damage `> 0`,
/// the SAME value that gates §6 severity) is frozen here too; the fold owns no
/// component attach (the fire path's `apply_dot` boundary does). Mutates the target
/// ganger's battle state in place; owns no mutation after return.
#[expect(
    clippy::too_many_arguments,
    reason = "GTW-438 threads the injury-roll inputs (the InjuryTables + InjuryRegistry \
              reads + the &mut InjuryRng draw stream) onto the wound fold alongside the \
              irreducible outcome / weapon / luck / target / entity / tuning / severity-rng \
              set; the target ganger surfaces are ALREADY grouped in the TargetGanger \
              bundle. The §5 → §6 → §8 wound math itself is the shared synthesize_wound core \
              (GTW-523) — this is only the weapon-path input resolution + verdict freeze"
)]
pub(in crate::damage_resolution::resolve_and_apply) fn fold(
    outcome: &ShotOutcome,
    weapon: WeaponStats<'_>,
    shooter_luck: Luck,
    target: Option<TargetGanger<'_>>,
    target_entity: Entity,
    tuning: &CombatTuning,
    rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> HitVerdict {
    // A defensive `None` target (the struck entity was not a queryable ganger) folds to
    // no-effect — never a panic, no draw.
    let Some(target) = target else {
        return HitVerdict::NoEffect;
    };

    // The struck part rode along on the §4 part roll (drawn upstream); a Ganger outcome
    // carries Some. A defensive None folds to no-effect (no draw). This gate is the
    // weapon path's alone (the core takes a resolved BodyPart).
    let Some(part) = outcome.body_part else {
        return HitVerdict::NoEffect;
    };

    // Armored piece + matchup, or zeroed bare flesh under Neutral — the weapon path's own
    // input resolution. The struck piece is resolved by the caller from `ganger → Wears →
    // the BodyPart-tagged piece` (GTW-323 / ADR-0004); `struck_piece` reads its stats (the
    // wear mutation happens later, inside the shared core's `apply_hit`).
    let (piece, resolved_matchup) = struck_piece(target.piece.as_ref(), weapon);

    // The SHARED wound-synthesis core (GTW-523): corpse-skip → resolve_hit → roll_severity
    // (the ONE SeverityRng draw) → apply_hit → roll_injury (the ONE severity-gated
    // InjuryRng draw). Returns None on the corpse-skip (no draw, no mutation) — the ganger
    // fold then yields no effect, exactly as the old inline corpse-skip did.
    let Some(synthesis) = synthesize_wound(WoundCoreInputs {
        blow: WoundBlow {
            part,
            damage: *weapon.damage,
            punch: *weapon.punch,
            shred: *weapon.shred,
            piece,
            matchup: resolved_matchup,
            fatal_bias: *weapon.fatal_bias,
            shooter_luck,
        },
        target,
        target_entity,
        tuning,
        severity_rng: rng,
        tables,
        registry,
        injury_rng,
    }) else {
        return HitVerdict::NoEffect;
    };

    // GTW-544: the DOT-attach decision. The firing weapon carries a `DotProfile` AND this
    // hit PENETRATED armor (the pre-floor `PenetratingDamage > 0`, the SAME value that gates
    // §6 severity) ⇒ freeze a `Dot` built from the profile onto the verdict. A fully-soaked
    // hit (penetrating `0`, HP may still bruise) and a non-DOT weapon both freeze nothing.
    let dot_applied = weapon
        .dot
        .filter(|_| *synthesis.hit.penetrating > 0)
        .map(|profile| Dot::from_profile(*profile));

    // Freeze the verdict — named newtypes + the rolled injury, no pixel. Every field is
    // read straight off the shared core's WoundSynthesis (the matchup, hit, severity, wear,
    // post-hit life, and injury), so there is no parallel wound math here. The wear outcome
    // rides the closed ArmorWearOutcome DIRECTLY (GTW-573 C2 — never re-flattened into a
    // prose-exclusive Option pair).
    HitVerdict::Ganger(Box::new(GangerVerdict {
        target: target_entity,
        part,
        applied: AppliedDamage {
            matchup:    synthesis.matchup,
            hit:        synthesis.hit,
            severity:   synthesis.severity,
            life_after: synthesis.life_after,
            wear:       synthesis.wear,
        },
        injury: synthesis.injury,
        dot_applied,
    }))
}
