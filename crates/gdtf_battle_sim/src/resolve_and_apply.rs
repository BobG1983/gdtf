//! The E3.9 **capstone integrator** — `resolve_and_apply` folds one
//! [`ShotOutcome`] → damage → severity → application into ONE model-side act and
//! returns a FROZEN per-hit report.
//!
//! This is the last E3 slice (`docs/combat/resolution.md` §5 / §6 / §"What's pure
//! math vs sim"; `docs/combat/weapons-and-armor.md` §"Per-hit resolution"). It
//! applies the result as ONE model-side act (armor → severity → application, with
//! corpse-skip draw discipline) and returns the frozen per-round reports — the
//! authoritative-model role this crate plays in the model/view split (ADR-0001,
//! `docs/decisions/0001-rust-bevy-rewrite.md`). It **composes the already-built E3
//! verbs** — it rebuilds none of them:
//!
//! 1. **Kind gate** — only a [`ShotKind::Ganger`] outcome can wound. A
//!    [`ShotKind::Cover`] / [`ShotKind::Slab`] / [`ShotKind::Ground`] /
//!    [`ShotKind::Miss`] outcome folds to a **no-damage** report — no wound, and
//!    **no RNG draw** (a non-ganger never touches a ganger).
//! 2. **Corpse-skip — BEFORE any draw** — a target already at
//!    [`LifeState::Dead`] yields a **no-effect** report: nothing mutates and **no
//!    draw is taken**, so a corpse can never consume an RNG draw (determinism is
//!    preserved) and a later round in a burst cannot re-wound a corpse
//!    (resolution.md §9's corpse-skip discipline).
//! 3. **Part** — the struck [`BodyPart`] is the outcome's
//!    [`ShotOutcome::body_part`] (the §4 location roll already drawn upstream). A
//!    `Ganger` outcome carries `Some`; a defensive `None` folds to a no-damage
//!    report.
//! 4. **Armor lookup (or bare flesh)** — if the worn piece at the struck part
//!    still [`protects`](WornArmor::protects), the hit resolves against that
//!    [`ArmorPiece`] under the E3.2 [`matchup`] of the weapon's [`DamageType`] vs
//!    the piece's [`ArmorType`]. Otherwise it resolves as **bare flesh**: a zeroed
//!    soak (floor / protection / hardness all `0`) under [`Matchup::Neutral`]
//!    (there is no armor type to match against, so no wheel advantage —
//!    `weapons-and-armor.md` §"Per-hit resolution": "later hits on that location
//!    resolve as bare flesh").
//! 5. **Damage** — E3.3 [`resolve_hit`] → [`HitResult`] against that piece + matchup.
//! 6. **Severity — the ONE draw** — E3.4 [`roll_severity`] over the
//!    [`SeverityInputs`] assembled from the hit's penetrating damage, the
//!    defender's [`Toughness`], the struck part's [`part_severity_mod`], the
//!    weapon's [`FatalBias`], and **both** gangers' [`Luck`]. The injected
//!    [`SimRng`] is the **single draw point** — no `thread_rng`, no ad-hoc entropy.
//! 7. **Apply** — E3.6 [`apply_hit`] folds HP loss + Wounds-by-tier + armor wear +
//!    the terminal gates onto the target in place, surfacing the
//!    `Some(`[`ArmorBroken`]`)` on a protecting→broken crossing.
//! 8. **Freeze** — the returned [`HitReport`] is a `Copy` record of named newtypes
//!    (the matchup, the [`HitResult`], the [`Severity`], the resulting
//!    [`LifeState`], the optional [`ArmorBroken`], the struck [`BodyPart`]) for the
//!    presenter's FX. `resolve_and_apply` owns **no** mutation after return.
//!
//! `resolve_and_apply` is the MODEL-side integrator. It does **not** charge TU or
//! loop the burst — that is the E4 `fire()` act (resolution.md §"What's pure math
//! vs sim"). Pure, render-free model logic: it carries **no pixel** — the report
//! holds only damage / wound math, never a screen coordinate.

use bevy::prelude::Entity;

use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        BodyPart, WornArmor,
    },
    armor_wear::ArmorBroken,
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    matchup::{Matchup, matchup},
    resolve_coarse::{ShotKind, ShotOutcome},
    resolve_hit::{HitResult, resolve_hit},
    rng::SimRng,
    severity::{Severity, SeverityInputs, part_severity_mod, roll_severity},
    tuning::CombatTuning,
    weapon::WeaponStats,
};

/// The **bundle of one target ganger's battle state** [`resolve_and_apply`] folds a
/// hit onto — the four `&mut` battle surfaces a hit can change, plus the two read
/// attribute stats the severity roll needs.
///
/// Grouping these into one named struct keeps [`resolve_and_apply`] under clippy's
/// argument-count gate (the [`GangerHitTarget`] / [`SeverityInputs`] precedent). The
/// four mutable borrows are exactly the [`GangerHitTarget`] set (assembled from this
/// bundle when [`apply_hit`] runs); the two reads ([`Toughness`] / [`Luck`]) are the
/// E3.0 defender attribute components fed to the severity roll. Every field is an
/// existing named domain component (no-bare-types). The caller (a Bevy system, or
/// the E4 `fire()` act) assembles this from the target entity's components.
pub struct TargetGanger<'a> {
    /// The target's hit-points pool — the HP loss subtracts from it (always).
    pub hp:        &'a mut Hp,
    /// The target's Wounds (life) pool — the severity tier spends from it.
    pub wounds:    &'a mut Wounds,
    /// The target's terminal life state — the gates set it; corpse-skip reads it.
    pub life:      &'a mut LifeState,
    /// The target's battle-local worn armor — the struck piece wears in place; its
    /// [`protects`](WornArmor::protects) decides the armored-vs-bare-flesh branch.
    pub worn:      &'a mut WornArmor,
    /// The target's Toughness — the defender's severity-mitigation term (E3.0, read).
    pub toughness: Toughness,
    /// The target's Luck — extends the severity roll's floor down (E3.0, read).
    pub luck:      Luck,
}

/// The **applied-damage block** of a [`HitReport`] — the resolved damage of a hit
/// that landed on a ganger (`docs/combat/resolution.md` §5 / §6).
///
/// A frozen `Copy` record of named newtypes (no bare primitive, no pixel): the
/// resolved [`Matchup`], the per-hit [`HitResult`], the rolled [`Severity`], the
/// ganger's [`LifeState`] **after** application, and the `Some(`[`ArmorBroken`]`)`
/// iff this hit broke the struck piece. The presenter reads it for FX; it is never
/// mutated after [`resolve_and_apply`] returns. Present only when the hit actually
/// landed on a ganger — a non-ganger / corpse-skip / no-part report carries `None`
/// in [`HitReport::applied`].
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
    pub broken:     Option<ArmorBroken>,
}

/// The **frozen per-hit report** [`resolve_and_apply`] returns — the entire E3.9
/// fold's verdict (the frozen per-round report the authoritative model hands the
/// view; ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A `Copy` value object of named domain types (no bare primitive, **no pixel** —
/// it carries only damage / wound math, never a screen coordinate). The presenter
/// reads it for FX staging; [`resolve_and_apply`] owns no mutation after it is
/// returned.
///
/// - [`kind`](HitReport::kind) — what the shot struck (the [`ShotOutcome`]'s
///   [`ShotKind`], carrying the struck ganger / cover / surface-cell payload).
/// - [`part`](HitReport::part) — the struck [`BodyPart`], `Some` only for a hit
///   that landed on a ganger.
/// - [`applied`](HitReport::applied) — the [`AppliedDamage`] block, `Some` only
///   for a hit that landed on a ganger; `None` for a non-ganger outcome, a
///   corpse-skip, or a defensively-missing part (a **no-effect** report).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitReport {
    /// What the shot struck — the [`ShotOutcome`]'s [`ShotKind`].
    pub kind:    ShotKind,
    /// The struck [`BodyPart`] — `Some` only when the hit landed on a ganger.
    pub part:    Option<BodyPart>,
    /// The applied-damage block — `Some` only when the hit landed on a ganger;
    /// `None` is a no-effect report (non-ganger / corpse-skip / no-part).
    pub applied: Option<AppliedDamage>,
}

impl HitReport {
    /// Build a **no-effect** report for `kind` — no part struck and no damage
    /// applied (the non-ganger, corpse-skip, and defensive-no-part folds).
    ///
    /// `pub` so the E4.5 `fire()` act (GTW-198) can fold a non-ganger / corpse-skip
    /// round to a no-effect report cross-module without re-deriving the shape.
    #[must_use]
    pub const fn no_effect(kind: ShotKind) -> Self {
        Self {
            kind,
            part: None,
            applied: None,
        }
    }
}

/// The **bare-flesh** armor piece — a zeroed soak used when the struck location no
/// longer [`protects`](WornArmor::protects) (`weapons-and-armor.md` §"Per-hit
/// resolution": "later hits on that location resolve as bare flesh").
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

/// Resolve the struck location to the `(`[`ArmorPiece`]`, `[`Matchup`]`)` the
/// per-hit formula runs against — the armored branch or **bare flesh**.
///
/// If the worn piece at `part` still [`protects`](WornArmor::protects), returns that
/// piece and the E3.2 [`matchup`] of the weapon's [`DamageType`] vs the piece's
/// [`ArmorType`] (the wheel advantage applies). Otherwise returns the zeroed
/// [`BARE_FLESH`] piece under [`Matchup::Neutral`] — there is no armor type to match
/// against, so no wheel advantage, and the zeroed soak means the hit lands as full
/// weapon damage (`weapons-and-armor.md` §"Per-hit resolution").
fn struck_piece(
    worn: &WornArmor,
    part: BodyPart,
    weapon: WeaponStats<'_>,
) -> (ArmorPiece, Matchup) {
    if worn.protects(part) {
        let piece = worn.at(part);
        let resolved = matchup(*weapon.damage_type, piece.armor_type);
        (piece, resolved)
    } else {
        // Bare flesh: no protection / hardness, no armor type to match → Neutral.
        (BARE_FLESH, Matchup::Neutral)
    }
}

/// Fold **one [`ShotOutcome`]** through damage → severity → application into ONE
/// frozen [`HitReport`] — the E3.9 capstone integrator (`docs/combat/resolution.md`
/// §5 / §6).
///
/// Composes the already-built E3 verbs (E3.2 [`matchup`] → E3.3 [`resolve_hit`] →
/// E3.4 [`roll_severity`] → E3.6 [`apply_hit`]); it rebuilds none of them. The
/// flow, in order:
///
/// 1. **Kind gate** — a non-[`ShotKind::Ganger`] outcome returns a no-effect
///    report with **no draw and no mutation** (a non-ganger never wounds a ganger).
/// 2. **Corpse-skip — before any draw** — a target already at [`LifeState::Dead`]
///    returns a no-effect report with **no draw and no mutation**, so a corpse
///    never consumes an RNG draw and determinism is preserved.
/// 3. **Part** — the struck [`BodyPart`] is [`ShotOutcome::body_part`] (drawn
///    upstream); a defensive `None` returns a no-effect report.
/// 4. **Armor / bare flesh** — [`struck_piece`] picks the armored piece + matchup
///    or the zeroed bare-flesh piece under [`Matchup::Neutral`].
/// 5. **Damage** — [`resolve_hit`] → [`HitResult`].
/// 6. **Severity (the ONE draw)** — [`roll_severity`] over the assembled
///    [`SeverityInputs`], drawing from the injected [`SimRng`] — the single draw
///    point (no `thread_rng`, no ad-hoc entropy).
/// 7. **Apply** — [`apply_hit`] folds the hit onto the target in place.
/// 8. **Freeze** — returns the [`HitReport`] of named newtypes.
///
/// Pure, render-free model logic. It mutates the target ganger's battle state in
/// place (and advances the injected [`SimRng`] by **exactly one** severity draw,
/// only on a ganger hit) and **owns no mutation after return**. Same
/// [`BattleSeed`](crate::rng::BattleSeed) → identical report for identical inputs
/// (the seeded-replay property). Charging TU and looping the burst is the E4
/// `fire()` act — **out of scope** here.
#[must_use]
pub fn resolve_and_apply(
    outcome: &ShotOutcome,
    weapon: WeaponStats<'_>,
    shooter_luck: Luck,
    target: TargetGanger<'_>,
    target_entity: Entity,
    tuning: &CombatTuning,
    rng: &mut SimRng,
) -> HitReport {
    // (1) Kind gate: only a Ganger outcome can wound. Non-ganger → no draw, no
    // mutation (the round struck cover / a slab / the ground / nothing — never a
    // ganger).
    let ShotKind::Ganger(_) = outcome.kind else {
        return HitReport::no_effect(outcome.kind);
    };

    // (2) Corpse-skip BEFORE any draw: a dead target is final — no draw is taken
    // (a corpse never consumes an RNG draw), nothing mutates.
    if *target.life == LifeState::Dead {
        return HitReport::no_effect(outcome.kind);
    }

    // (3) The struck part rode along on the §4 part roll (drawn upstream); a Ganger
    // outcome carries Some. A defensive None folds to a no-effect report (no draw).
    let Some(part) = outcome.body_part else {
        return HitReport::no_effect(outcome.kind);
    };

    // (4) Armored piece + matchup, or zeroed bare flesh under Neutral.
    let (piece, resolved_matchup) = struck_piece(target.worn, part, weapon);

    // (5) The per-hit damage formula (E3.3) — pure, mutates nothing.
    let hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        resolved_matchup,
        tuning,
    );

    // (6) The ONE RNG draw: the wound-severity roll (E3.4). Both gangers' Luck, the
    // defender's Toughness, the struck part's mod, and the weapon's fatal bias feed
    // it; the injected SimRng is the single draw point.
    let inputs = SeverityInputs::new(
        hit.penetrating,
        target.toughness,
        part_severity_mod(part),
        *weapon.fatal_bias,
        shooter_luck,
        target.luck,
    );
    let severity = roll_severity(&inputs, &tuning.severity_scaling, rng);

    // (7) Apply the resolved hit onto the target in place (E3.6) — HP loss +
    // Wounds-by-tier + armor wear + the terminal gates; capture the broken signal.
    let broken = apply_hit(
        GangerHitTarget {
            hp:     target.hp,
            wounds: target.wounds,
            life:   target.life,
            worn:   target.worn,
        },
        &hit,
        severity,
        part,
        target_entity,
        tuning,
    );

    // (8) Freeze the verdict — a Copy record of named newtypes, no pixel.
    HitReport {
        kind:    outcome.kind,
        part:    Some(part),
        applied: Some(AppliedDamage {
            matchup: resolved_matchup,
            hit,
            severity,
            life_after: *target.life,
            broken,
        }),
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{Entity, World};

    use super::*;
    use crate::{
        armor::SourceArmor,
        central_axis::climb_aim_dir,
        cone::{ConeAngle, PriorShots},
        cover::{CoverEntry, CoverHp, HeightBand},
        metric::{Cell, CellLevel, Level, SimPos},
        resolve_coarse::ShotKind,
        rng::BattleSeed,
        sample_cone::{ConcentrationP, ShotDir, sample_cone_vector},
        stability::RecoilGrowth,
        tuning::RecoilClimb,
        weapon::{
            Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
            HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
            ModeTuPercent, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
            WeaponShred,
        },
    };

    /// A fixed seed for the per-test RNG streams — determinism is a property, so the
    /// same seed must reproduce the same draws (an arbitrary value, not tuned).
    const SEED: u64 = 0x05EE_D191;

    /// Build a `SimRng` from the shared fixed seed (a fresh stream per call).
    fn rng() -> SimRng {
        SimRng::from_seed(BattleSeed::new(SEED))
    }

    /// A real, valid [`Entity`] id to stand in for a ganger — spawned from a
    /// throwaway [`World`] so the tests never hand-craft a raw id (no `unwrap`).
    fn an_entity() -> Entity {
        World::new().spawn_empty().id()
    }

    /// An armed-entity bundle built from arbitrary (NOT shipped-tuning) magnitudes
    /// — only the per-hit damage stats and the damage type matter to these tests;
    /// the §1 cone/recoil numbers and the fire mode are present but irrelevant here.
    /// Returns the owned [`WeaponBundle`]; the call site assembles the
    /// [`WeaponStats`] read-view via [`WeaponBundle::stats`].
    fn a_weapon(damage: i32, punch: i32, shred: i32, damage_type: DamageType) -> WeaponBundle {
        let spec = FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(1.0),
            ModeShots::new(1),
        );
        WeaponBundle::new(
            WeaponName::new("test-weapon".to_owned()),
            BaseSpread::new(0.1),
            Accuracy::new(1.0),
            Kickback::new(0.0),
            FatalBias::new(0.0),
            DamageProfile::new(
                WeaponDamage::new(damage),
                WeaponPunch::new(punch),
                WeaponShred::new(shred),
                damage_type,
            ),
            HandlingProfile::new(
                MagazineSize::new(10),
                FireMode::new(vec![spec]),
                Stable::new(false),
            ),
        )
    }

    /// A uniform worn suit whose every piece starts at the given stats — arbitrary
    /// (NOT shipped-tuning) magnitudes, so a hit lands in a known regime.
    fn worn_suit(
        floor: i32,
        protection: i32,
        integrity: i32,
        hardness: i32,
        armor_type: ArmorType,
    ) -> WornArmor {
        WornArmor::seed_from(&SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(floor),
            ArmorProtection::new(protection),
            ArmorIntegrity::new(integrity),
            ArmorHardness::new(hardness),
            armor_type,
        )))
    }

    /// A unit-direction [`ShotDir`] fixture — minted through the REAL pipeline
    /// ([`climb_aim_dir`] → [`sample_cone_vector`] with a zero cone, which returns
    /// the axis exactly and draws nothing), since [`ShotDir`] has no test
    /// constructor. The direction is arbitrary (E3.9 never reads `trajectory`); a
    /// sim-unit axis, **zero pixels**.
    fn a_trajectory() -> ShotDir {
        let aim = climb_aim_dir(
            SimPos::new(0.0, 0.0, 0.5),
            SimPos::new(5.0, 0.0, 0.5),
            PriorShots::new(0),
            RecoilClimb::new(0.0),
            RecoilGrowth::new(0.0),
        );
        // A zero cone short-circuits to the axis exactly and consumes no draw.
        sample_cone_vector(
            aim,
            ConeAngle::new(0.0),
            ConcentrationP::new(1.0),
            rng().rng(),
        )
    }

    /// A `ShotKind::Ganger` outcome on `entity`, struck at `part`. The cell / band /
    /// muzzle / trajectory fields are present-but-irrelevant to E3.9 (it reads only
    /// `kind` + `body_part`), so they carry arbitrary sim-unit fixtures (zero px).
    fn ganger_outcome(entity: Entity, part: BodyPart) -> ShotOutcome {
        ShotOutcome {
            kind:       ShotKind::Ganger(entity),
            cell:       Cell::new(3, 4),
            level:      Level::new(0),
            body_part:  Some(part),
            band:       HeightBand::Mid,
            muzzle:     SimPos::new(0.5, 0.5, 0.5),
            trajectory: a_trajectory(),
        }
    }

    /// A non-ganger outcome of the given `kind` — no body part (the §4 roll runs
    /// only on a ganger). The cell / band / muzzle / trajectory are arbitrary.
    fn non_ganger_outcome(kind: ShotKind) -> ShotOutcome {
        ShotOutcome {
            kind,
            cell: Cell::new(1, 1),
            level: Level::new(0),
            body_part: None,
            band: HeightBand::Low,
            muzzle: SimPos::new(0.5, 0.5, 0.5),
            trajectory: a_trajectory(),
        }
    }

    /// AC1 (THE key test) — the fold equals the composition: `resolve_and_apply` on
    /// a `Ganger` outcome produces a report whose damage / severity AND the
    /// resulting ganger state are IDENTICAL to running `matchup` → `resolve_hit` →
    /// `roll_severity` → `apply_hit` by hand, with the SAME seed and the SAME inputs
    /// on a clone of the target. The units under test are NOT shadowed — both sides
    /// call the real verbs; the same single draw makes the RNG streams line up.
    #[test]
    fn fold_equals_the_composed_steps() {
        let tuning = CombatTuning::default();
        let entity = an_entity();
        let part = BodyPart::Torso;
        let weapon = a_weapon(14, 10, 6, DamageType::Kinetic);
        let shooter_luck = Luck::new(2.0);
        let outcome = ganger_outcome(entity, part);

        // --- The folded act ---
        let mut hp_a = Hp::new(40);
        let mut wounds_a = Wounds::new(6);
        let mut life_a = LifeState::Alive;
        let mut worn_a = worn_suit(1, 8, 30, 2, ArmorType::Void);
        let toughness = Toughness::new(3.0);
        let defender_luck = Luck::new(4.0);
        let mut rng_a = rng();
        let report = resolve_and_apply(
            &outcome,
            weapon.stats(),
            shooter_luck,
            TargetGanger {
                hp: &mut hp_a,
                wounds: &mut wounds_a,
                life: &mut life_a,
                worn: &mut worn_a,
                toughness,
                luck: defender_luck,
            },
            entity,
            &tuning,
            &mut rng_a,
        );

        // --- The composed steps, BY HAND, on a clone with the same seed ---
        let mut hp_b = Hp::new(40);
        let mut wounds_b = Wounds::new(6);
        let mut life_b = LifeState::Alive;
        let mut worn_b = worn_suit(1, 8, 30, 2, ArmorType::Void);
        let mut rng_b = rng();

        let piece = worn_b.at(part);
        let m = matchup(weapon.damage_type, piece.armor_type);
        let hit = resolve_hit(
            weapon.damage,
            weapon.punch,
            weapon.shred,
            &piece,
            m,
            &tuning,
        );
        let inputs = SeverityInputs::new(
            hit.penetrating,
            toughness,
            part_severity_mod(part),
            weapon.fatal_bias,
            shooter_luck,
            defender_luck,
        );
        let severity = roll_severity(&inputs, &tuning.severity_scaling, &mut rng_b);
        let broken = apply_hit(
            GangerHitTarget {
                hp:     &mut hp_b,
                wounds: &mut wounds_b,
                life:   &mut life_b,
                worn:   &mut worn_b,
            },
            &hit,
            severity,
            part,
            entity,
            &tuning,
        );

        // The report's damage block matches the hand-composed steps.
        assert_eq!(
            report.applied,
            Some(AppliedDamage {
                matchup: m,
                hit,
                severity,
                life_after: life_b,
                broken,
            }),
            "the folded report must equal the composed matchup/hit/severity/state",
        );
        assert_eq!(
            report.part,
            Some(part),
            "the report carries the struck part"
        );

        // The resulting ganger state matches the hand-composed steps, surface by
        // surface — the fold mutated the target identically to the composition.
        assert_eq!(hp_a, hp_b, "HP after the fold must equal the composed HP");
        assert_eq!(
            wounds_a, wounds_b,
            "Wounds after the fold must equal the composed Wounds",
        );
        assert_eq!(
            life_a, life_b,
            "LifeState after the fold must equal the composed LifeState",
        );
        assert_eq!(
            worn_a, worn_b,
            "WornArmor after the fold must equal the composed WornArmor",
        );
    }

    /// AC2 — corpse-skip: a `Ganger` outcome on an already-`Dead` target yields a
    /// no-effect report and mutates nothing; AND it draws nothing — proven by the
    /// `SimRng` stream being unchanged vs a fresh one after the call.
    #[test]
    fn corpse_skip_is_inert_and_draws_nothing() {
        let tuning = CombatTuning::default();
        let entity = an_entity();
        let part = BodyPart::Head;
        let weapon = a_weapon(20, 12, 8, DamageType::Plasma);
        let outcome = ganger_outcome(entity, part);

        let mut hp = Hp::new(15);
        let mut wounds = Wounds::new(3);
        let mut life = LifeState::Dead; // already a corpse
        let mut worn = worn_suit(0, 0, 1, 0, ArmorType::DEFAULT); // a live hit WOULD break it

        let hp_before = hp;
        let wounds_before = wounds;
        let life_before = life;
        let worn_before = worn;

        let mut rng_used = rng();
        let report = resolve_and_apply(
            &outcome,
            weapon.stats(),
            Luck::new(5.0),
            TargetGanger {
                hp:        &mut hp,
                wounds:    &mut wounds,
                life:      &mut life,
                worn:      &mut worn,
                toughness: Toughness::new(2.0),
                luck:      Luck::new(1.0),
            },
            entity,
            &tuning,
            &mut rng_used,
        );

        // No-effect report.
        assert_eq!(report.applied, None, "a corpse-skip must apply no damage");
        assert_eq!(report.part, None, "a corpse-skip report carries no part");
        assert_eq!(
            report.kind,
            ShotKind::Ganger(entity),
            "the report still names what the shot struck",
        );

        // Nothing mutated.
        assert_eq!(hp, hp_before, "a corpse's Hp must not change");
        assert_eq!(wounds, wounds_before, "a corpse's Wounds must not change");
        assert_eq!(life, life_before, "a corpse's LifeState must stay Dead");
        assert_eq!(worn, worn_before, "a corpse's WornArmor must not wear");

        // No draw was taken: the used RNG's next draw matches a fresh stream's
        // first draw (the cursor never advanced).
        let mut rng_fresh = rng();
        assert_eq!(
            rng_used.next_u64(),
            rng_fresh.next_u64(),
            "corpse-skip must take NO draw — the SimRng cursor must be unadvanced",
        );
    }

    /// AC3 — bare-flesh path: wearing the struck piece to integrity ≤ 0 first
    /// (`protects(part) == false`) makes the hit resolve with NO protection /
    /// hardness. Asserts the report's matchup is `Neutral` (no armor type to match)
    /// AND the HP-loss equals the FULL weapon damage (zeroed soak) — distinct from
    /// the armored case on the same weapon, where protection soaks.
    #[test]
    fn bare_flesh_uses_no_protection_or_hardness() {
        let tuning = CombatTuning::default();
        let entity = an_entity();
        let part = BodyPart::RightArm;
        // A weapon whose damage is small enough that real protection WOULD soak it
        // below itself — so the bare-flesh full-damage result is unmistakable.
        let weapon = a_weapon(10, 3, 0, DamageType::Kinetic);

        // --- Bare flesh: the struck piece is worn through (integrity 0). ---
        let mut hp = Hp::new(50);
        let mut wounds = Wounds::new(9);
        let mut life = LifeState::Alive;
        // integrity 0 ⇒ protects(part) == false ⇒ bare flesh, despite real
        // protection/hardness numbers on the (broken) piece.
        let mut worn = worn_suit(2, 20, 0, 5, ArmorType::Void);
        assert!(
            !worn.protects(part),
            "fixture: the struck piece must already be worn through (no protection)",
        );

        let report = resolve_and_apply(
            &ganger_outcome(entity, part),
            weapon.stats(),
            Luck::new(0.0),
            TargetGanger {
                hp:        &mut hp,
                wounds:    &mut wounds,
                life:      &mut life,
                worn:      &mut worn,
                toughness: Toughness::new(0.0),
                luck:      Luck::new(0.0),
            },
            entity,
            &tuning,
            &mut rng(),
        );

        assert!(
            report.applied.is_some(),
            "a ganger hit must carry an applied-damage block",
        );
        let Some(applied) = report.applied else {
            return;
        };
        assert_eq!(
            applied.matchup,
            Matchup::Neutral,
            "bare flesh has no armor type to match → Neutral (no wheel advantage)",
        );
        // Zeroed soak ⇒ HP-loss == full weapon damage (no protection eats it).
        assert_eq!(
            *applied.hit.hp_damage, *weapon.damage,
            "bare flesh deals full weapon damage (no protection / hardness)",
        );
        assert_eq!(
            applied.broken, None,
            "bare flesh wears no piece — there is nothing to break",
        );

        // --- Armored counterpart: the same weapon vs a protecting piece soaks. ---
        let mut hp2 = Hp::new(50);
        let mut wounds2 = Wounds::new(9);
        let mut life2 = LifeState::Alive;
        // High protection, intact (protects == true) ⇒ the soak is in play.
        let mut worn2 = worn_suit(2, 20, 50, 5, ArmorType::Void);
        assert!(
            worn2.protects(part),
            "fixture: the armored piece must still protect",
        );
        let report2 = resolve_and_apply(
            &ganger_outcome(entity, part),
            weapon.stats(),
            Luck::new(0.0),
            TargetGanger {
                hp:        &mut hp2,
                wounds:    &mut wounds2,
                life:      &mut life2,
                worn:      &mut worn2,
                toughness: Toughness::new(0.0),
                luck:      Luck::new(0.0),
            },
            entity,
            &tuning,
            &mut rng(),
        );
        assert!(
            report2.applied.is_some(),
            "the armored hit must carry an applied-damage block",
        );
        let Some(applied2) = report2.applied else {
            return;
        };
        // The armored HP-loss is strictly below the bare-flesh full damage: real
        // protection soaked it. (Pins that bare flesh truly bypassed the soak.)
        assert!(
            *applied2.hit.hp_damage < *applied.hit.hp_damage,
            "the armored hit must take less HP-loss than bare flesh: {} >= {}",
            *applied2.hit.hp_damage,
            *applied.hit.hp_damage,
        );
    }

    /// AC4 — non-ganger outcomes are inert on a ganger: each of Cover / Slab /
    /// Ground / Miss yields a no-damage report and leaves the ganger's pools and
    /// state untouched (and takes no draw).
    #[test]
    fn non_ganger_outcomes_are_inert() {
        let tuning = CombatTuning::default();
        let weapon = a_weapon(50, 50, 50, DamageType::Rend);

        // Each non-ganger kind. AC4 enumerates Cover / Slab / Ground / Miss
        // explicitly, so all four are swept here. Cover carries a struck
        // CoverEntry fixture (seeded full-HP); Slab/Ground carry a struck cell;
        // Miss carries nothing. E3.9's early-return branches only on the variant
        // (`let ShotKind::Ganger(_) = ... else`), so every one must be inert.
        let kinds = [
            ShotKind::Miss,
            ShotKind::Cover(CoverEntry::seeded(
                CoverHp::new(40),
                HeightBand::Mid,
                ArmorProtection::new(3),
                ArmorHardness::new(2),
            )),
            ShotKind::Slab(CellLevel::new(Cell::new(2, 2), Level::new(1))),
            ShotKind::Ground(CellLevel::new(Cell::new(2, 2), Level::new(0))),
        ];

        for kind in kinds {
            let mut hp = Hp::new(30);
            let mut wounds = Wounds::new(5);
            let mut life = LifeState::Alive;
            let mut worn = worn_suit(0, 0, 1, 0, ArmorType::DEFAULT);

            let hp_before = hp;
            let wounds_before = wounds;
            let life_before = life;
            let worn_before = worn;

            let mut rng_used = rng();
            let report = resolve_and_apply(
                &non_ganger_outcome(kind),
                weapon.stats(),
                Luck::new(3.0),
                TargetGanger {
                    hp:        &mut hp,
                    wounds:    &mut wounds,
                    life:      &mut life,
                    worn:      &mut worn,
                    toughness: Toughness::new(1.0),
                    luck:      Luck::new(1.0),
                },
                an_entity(),
                &tuning,
                &mut rng_used,
            );

            assert_eq!(
                report.applied, None,
                "a {kind:?} outcome must apply no damage to a ganger",
            );
            assert_eq!(report.part, None, "a {kind:?} report carries no part");
            assert_eq!(report.kind, kind, "the report still names the struck kind");

            assert_eq!(hp, hp_before, "{kind:?} must not change Hp");
            assert_eq!(wounds, wounds_before, "{kind:?} must not change Wounds");
            assert_eq!(life, life_before, "{kind:?} must not change LifeState");
            assert_eq!(worn, worn_before, "{kind:?} must not wear WornArmor");

            // No draw taken on a non-ganger outcome.
            let mut rng_fresh = rng();
            assert_eq!(
                rng_used.next_u64(),
                rng_fresh.next_u64(),
                "a {kind:?} outcome must take NO draw",
            );
        }
    }

    /// AC5 — seeded determinism: two `resolve_and_apply` runs from the same
    /// `BattleSeed`, over the same outcome SEQUENCE on identical fresh targets,
    /// produce identical report sequences (replay equality). Walks a sequence so the
    /// stream — not just a single draw — is reproduced.
    #[test]
    fn same_seed_reproduces_the_report_sequence() {
        let tuning = CombatTuning::default();
        let entity = an_entity();
        let weapon = a_weapon(16, 9, 4, DamageType::Blast);
        // A sequence of struck parts so the stream is genuinely walked across calls.
        let parts = [
            BodyPart::Head,
            BodyPart::Torso,
            BodyPart::LeftArm,
            BodyPart::RightLeg,
            BodyPart::Torso,
        ];

        let run = || {
            let mut r = SimRng::from_seed(BattleSeed::new(SEED));
            // A fresh target per call so wear/state don't drift the comparison.
            let mut hp = Hp::new(60);
            let mut wounds = Wounds::new(12);
            let mut life = LifeState::Alive;
            let mut worn = worn_suit(1, 6, 40, 2, ArmorType::Flak);
            parts
                .iter()
                .map(|&part| {
                    resolve_and_apply(
                        &ganger_outcome(entity, part),
                        weapon.stats(),
                        Luck::new(1.0),
                        TargetGanger {
                            hp:        &mut hp,
                            wounds:    &mut wounds,
                            life:      &mut life,
                            worn:      &mut worn,
                            toughness: Toughness::new(2.0),
                            luck:      Luck::new(2.0),
                        },
                        entity,
                        &tuning,
                        &mut r,
                    )
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(
            run(),
            run(),
            "the same battle seed must reproduce the identical report sequence",
        );
    }

    /// AC6 (no-bare-types / frozen) — the report and its block are `Copy` records of
    /// named domain newtypes (no bare primitive), and the report is freely copyable.
    /// Pins the frozen-record shape (mechanism), never a tuning magnitude.
    #[test]
    fn report_is_a_frozen_record_of_named_newtypes() {
        let entity = an_entity();
        // A no-effect report copies and compares by value.
        let report = HitReport::no_effect(ShotKind::Miss);
        let copied = report; // Copy, not a move
        assert_eq!(report, copied, "HitReport must be Copy + PartialEq");
        assert_eq!(report.applied, None);
        assert_eq!(report.part, None);

        // An applied block is a Copy record of named newtypes.
        let applied = AppliedDamage {
            matchup:    Matchup::Favorable,
            hit:        HitResult {
                penetrating: crate::resolve_hit::PenetratingDamage::new(7),
                hp_damage:   crate::resolve_hit::HpDamage::new(8),
                wear:        crate::resolve_hit::IntegrityWear::new(9),
            },
            severity:   Severity::Major,
            life_after: LifeState::Downed,
            broken:     Some(ArmorBroken::new(entity, BodyPart::Torso)),
        };
        let applied_copy = applied; // Copy
        assert_eq!(
            applied, applied_copy,
            "AppliedDamage must be Copy + PartialEq"
        );
        // The block's fields are the named domain types (Deref reaches their inner).
        assert_eq!(*applied.hit.hp_damage, 8i32);
        assert_eq!(applied.severity, Severity::Major);
        assert_eq!(applied.matchup, Matchup::Favorable);
    }

    /// AC1 (counterpart) — the matchup wheel advantage IS felt on an armored ganger
    /// hit: at identical seed and inputs, a Favorable damage-type-vs-armor pairing
    /// yields `>=` penetrating damage than a Resisted one — the report's matchup is
    /// genuinely the wheel lookup, not a hardcoded Neutral.
    #[test]
    fn armored_report_carries_the_real_matchup() {
        let tuning = CombatTuning::default();
        let entity = an_entity();
        let part = BodyPart::Torso;

        // Pick a damage type/armor type pairing and confirm the report names the
        // wheel's verdict — then a clearly Resisted pairing names Resisted.
        let resolve = |weapon: WeaponBundle, armor_type: ArmorType| {
            let mut hp = Hp::new(50);
            let mut wounds = Wounds::new(9);
            let mut life = LifeState::Alive;
            let mut worn = worn_suit(1, 10, 40, 1, armor_type);
            resolve_and_apply(
                &ganger_outcome(entity, part),
                weapon.stats(),
                Luck::new(0.0),
                TargetGanger {
                    hp:        &mut hp,
                    wounds:    &mut wounds,
                    life:      &mut life,
                    worn:      &mut worn,
                    toughness: Toughness::new(0.0),
                    luck:      Luck::new(0.0),
                },
                entity,
                &tuning,
                &mut rng(),
            )
        };

        // Kinetic (node 3) is strong against {6, 1, 2} = {Ceramic, Refractive, Flak}
        // and resisted by the rest. Favorable vs Refractive, Resisted vs Void(3's
        // own mirror is Neutral, so use Plated node 0 → resisted).
        let weapon = a_weapon(12, 10, 6, DamageType::Kinetic);
        let fav = resolve(weapon.clone(), ArmorType::Refractive);
        let res = resolve(weapon, ArmorType::Plated);

        assert!(
            fav.applied.is_some() && res.applied.is_some(),
            "both armored hits must carry an applied block",
        );
        let (Some(fav_a), Some(res_a)) = (fav.applied, res.applied) else {
            return;
        };
        assert_eq!(
            fav_a.matchup,
            Matchup::Favorable,
            "Kinetic vs Refractive must resolve Favorable in the report",
        );
        assert!(
            *fav_a.hit.penetrating >= *res_a.hit.penetrating,
            "a Favorable matchup must yield >= penetrating damage than Resisted",
        );
    }
}
