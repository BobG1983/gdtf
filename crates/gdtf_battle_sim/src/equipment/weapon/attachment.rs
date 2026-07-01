//! Weapon **attachments** (GTW-542, child GTW-41b of GTW-41) — the closed
//! [`AttachTag`] enum a weapon authors in its `attachment_slots:` list, and the per-tag
//! FOLDER-FUNCTION dispatch ([`AttachmentEffects`]) that resolves the slot list into
//! spawn-side effects at the wielded-weapon scene seam.
//!
//! ## The model (ADR-0004)
//!
//! Weapon modding/attachments become "a sub-hierarchy hanging off the weapon entity"
//! (`docs/decisions/0004-equipment-as-entities-relationships.md`). This slice models
//! each attachment's *effect* as either a SIBLING COMPONENT on the weapon entity (the
//! [`Scoped`](super::Scoped) / [`Silenced`](super::Silenced) boolean tags, the
//! [`Stable`](super::Stable) / [`Shove`](super::Shove) precedent) OR a REWRITE of a
//! spawn-side weapon leaf (the damage / punch / magazine / fire-mode numbers the weapon
//! spawns with). It introduces NO net-new subsystem: every effect rides an EXISTING
//! hook (the additive [`SightStability`](crate::stability::SightStability) seam, the
//! GTW-526/GTW-468 producer gates, the §1/§6 weapon numbers), so the sim's rules do not
//! branch on attachments — they only see the resulting components / leaves.
//!
//! ## The dispatch
//!
//! [`AttachmentEffects::from_slots`] folds a weapon's `attachment_slots` list into (a) a
//! set of `spawn-side leaf` mutations applied IN PLACE to the resolved weapon bundle, and
//! (b) the optional
//! [`Scoped`](super::Scoped) / [`Silenced`](super::Silenced) sibling components the
//! spawn seam tuple-composes onto the weapon entity. An EMPTY slot list folds to the
//! identity — no leaf changes, no siblings — so a weapon that authors no attachments
//! spawns BYTE-IDENTICAL to before this slice existed (the `#[serde(default)]` opt-in
//! precedent of [`Shove`](super::Shove)).

use bevy::prelude::{Entity, Query, With};
use serde::Deserialize;

use super::{
    DamageType, FireMode, MagazineSize, Scoped, Silenced, WeaponBundle, WeaponDamage, WeaponPunch,
    WeaponSightBonus,
};
use crate::{
    fire::{MeleeQuery, WieldsQuery},
    magazine::{Magazine, ReloadTu},
    tuning::{
        AttachmentTuning, BraceBonus, ConeMultDelta, ReloadDelta, ReloadFactor, SpreadPenalty,
    },
    weapon::{FatalBias, ModeConeMult, ModeTuPercent},
};

/// A **weapon-attachment slot** — one fitted attachment a weapon carries (GTW-542,
/// child GTW-41b). A closed, named domain enum: each variant is a distinct modding kit,
/// and every magnitude-carrying variant holds its magnitude as a NAMED-newtype payload
/// (no bare primitive), reusing an existing weapon / tuning newtype where one fits or a
/// dedicated attachment-tuning leaf otherwise.
///
/// A weapon authors zero or more of these in its `attachment_slots:` `.weapon.ron` list
/// ([`WeaponSpec::attachment_slots`](super::WeaponSpec::attachment_slots)); the
/// spawn-side [`AttachmentEffects`] dispatch folds each into a sibling component OR a
/// spawn-side leaf rewrite (see the module doc). `#[derive(Deserialize)]` so the list
/// round-trips from RON by variant name; NOT a `Component` — it is authoring DATA the
/// folder-functions consume, not a live weapon-entity component.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum AttachTag {
    // ── The 4 NAMED, locked effects ──────────────────────────────────────────────
    /// **Stable** — fits the weapon with a bipod / brace, adding the
    /// [`Stable`](super::Stable)`(true)` tag (the §1a unconditional brace). Reuses the
    /// existing brace OR-read; no new path.
    Stable,
    /// **Sighted** — fits a precision optic, adding the [`Scoped`](super::Scoped)`(true)`
    /// tag so the §1a stability read folds in the additive
    /// [`SightStabilityBonus`](crate::tuning::SightStabilityBonus) (a tighter cone).
    Sighted,
    /// **Silenced** — fits a suppressor, adding the [`Silenced`](super::Silenced)`(true)`
    /// tag so the weapon's shots propagate neither SUPPRESSION nor REACTION/REVEAL (both
    /// producers gate on the tag).
    Silenced,
    /// **`FastReload`** — fits a speed-loader, MULTIPLYING the weapon's
    /// [`ReloadTu`](crate::magazine::ReloadTu) by the tuning
    /// [`fast_reload_factor`](crate::tuning::AttachmentTuning::fast_reload_factor) (`< 1`
    /// → a faster reload).
    FastReload,

    // ── The 8 GRIMDARK effects (defensible defaults) ─────────────────────────────
    /// **Hexgrind rounds** — cursed ammunition that bites deeper: ADDS `punch_bonus` to
    /// the weapon's [`WeaponPunch`](super::WeaponPunch) (more penetration).
    HexgrindRounds {
        /// The extra penetration the hexgrind rounds add (a [`WeaponPunch`](super::WeaponPunch)
        /// addend).
        punch_bonus: WeaponPunch,
    },
    /// **Rotgut coating** — a toxic coating that OVERRIDES the weapon's emitted
    /// [`DamageType`](super::DamageType) to `damage_type` (a chem/rend re-key).
    RotgutCoating {
        /// The damage type the coating forces the weapon to emit.
        damage_type: DamageType,
    },
    /// **Butcher's weight** — a brutal counterweight: ADDS `damage_bonus` to
    /// [`WeaponDamage`](super::WeaponDamage) AND ADDS `tu_percent_delta` to every fire
    /// mode's [`ModeTuPercent`](super::ModeTuPercent) (heavier hits, slower to bring to
    /// bear).
    ButchersWeight {
        /// The extra base damage the counterweight adds (a [`WeaponDamage`](super::WeaponDamage)
        /// addend).
        damage_bonus:     WeaponDamage,
        /// The extra per-mode TU-percent the counterweight adds (a heavier weapon fires
        /// slower).
        tu_percent_delta: ModeTuPercent,
    },
    /// **Whisper bore** — a machined bore that quiets the shot AND steadies it: adds the
    /// [`Silenced`](super::Silenced)`(true)` tag PLUS a small
    /// [`whisper_bore_sight`](crate::tuning::AttachmentTuning::whisper_bore_sight)
    /// stability bonus (a suppressor bundled with a modest optic).
    WhisperBore,
    /// **Sump-slick action** — a jury-rigged fast action: MULTIPLIES the reload cost by
    /// `reload_factor` (a smaller speed-up than [`FastReload`](AttachTag::FastReload)) at
    /// the cost of `spread_penalty` extra base spread (faster, sloppier).
    SumpSlickAction {
        /// The reload-time factor (`< 1` speeds the reload; smaller effect than [`FastReload`](AttachTag::FastReload)).
        reload_factor:  ReloadFactor,
        /// The extra base spread the jury-rigged action adds (a wider cone).
        spread_penalty: SpreadPenalty,
    },
    /// **Gore-sump drum** — an oversized magazine: ADDS `size_bonus` to the weapon's
    /// [`MagazineSize`](super::MagazineSize) AND ADDS `reload_delta` to its
    /// [`ReloadTu`](crate::magazine::ReloadTu) (more rounds, slower to swap).
    GoreSumpDrum {
        /// The extra magazine capacity the drum adds (a [`MagazineSize`](super::MagazineSize)
        /// addend).
        size_bonus:   MagazineSize,
        /// The extra reload cost the drum's bulk adds (a [`ReloadDelta`](crate::tuning::ReloadDelta)).
        reload_delta: ReloadDelta,
    },
    /// **Dead man's brace** — a rigid brace that steadies the aim: adds `brace_bonus`
    /// §1a stability-score points (fed as an additive
    /// [`SightStability`](crate::stability::SightStability) contribution, the same seam a
    /// [`Scoped`](super::Scoped) optic uses).
    DeadmansBrace {
        /// The stability-score points the brace adds (a [`BraceBonus`](crate::tuning::BraceBonus)).
        brace_bonus: BraceBonus,
    },
    /// **Executioner's choke** — a savage muzzle: ADDS `fatal_bias_bonus` to the weapon's
    /// [`FatalBias`](super::FatalBias) (nastier wounds) at the cost of `cone_mult_delta`
    /// extra per-mode cone multiplier (a wider, deadlier spray).
    ExecutionersChoke {
        /// The extra fatal-bias the choke adds (a [`FatalBias`](super::FatalBias) addend —
        /// nastier §6 wound buckets).
        fatal_bias_bonus: FatalBias,
        /// The extra per-mode cone multiplier the choke adds (a wider cone).
        cone_mult_delta:  ConeMultDelta,
    },
}

/// The resolved spawn-side **effects** of a weapon's `attachment_slots` list — the
/// accumulator [`AttachmentEffects::from_slots`] folds the slot list into, applied to the
/// resolved [`WeaponBundle`] at the wielded-weapon scene seam (GTW-542).
///
/// Splits an attachment's effect into (a) the SIBLING boolean tags the spawn seam
/// tuple-composes onto the weapon entity ([`sighted`](AttachmentEffects::sighted) /
/// [`silenced`](AttachmentEffects::silenced)) and (b) the spawn-side leaf mutations
/// [`from_slots`](AttachmentEffects::from_slots) folds into the bundle's own
/// numbers. An EMPTY slot list produces the identity (`sighted`/`silenced` both `None`,
/// no leaf change), so a weapon with no attachments spawns byte-identical.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AttachmentEffects {
    /// The [`Scoped`](super::Scoped) sibling to add, or `None` when no sight-bearing
    /// attachment is fitted.
    sighted:     Option<Scoped>,
    /// The [`Silenced`](super::Silenced) sibling to add, or `None` when no
    /// suppressor-bearing attachment is fitted.
    silenced:    Option<Silenced>,
    /// The per-weapon [`WeaponSightBonus`](super::WeaponSightBonus) override to add, or
    /// `None` when the fitted sight (if any) uses the universal tuning bonus. Only
    /// meaningful when [`sighted`](AttachmentEffects::sighted) is also `Some`.
    sight_bonus: Option<WeaponSightBonus>,
}

impl AttachmentEffects {
    /// Fold a weapon's `attachment_slots` list into its spawn-side effects, mutating the
    /// resolved `bundle`'s leaves in place and returning the sibling-tag accumulator.
    ///
    /// Each slot is dispatched to its per-tag folder-function (the `match` below): a tag
    /// either flips a sibling ([`Scoped`](super::Scoped) / [`Silenced`](super::Silenced))
    /// or rewrites a spawn-side leaf on `bundle` (damage / punch / fatal-bias / magazine /
    /// fire-mode / damage-type). The `sight`/`stable` §1a contributions and the
    /// producer-gate tags are the EXISTING hooks the effects land on — no attachment-only
    /// subsystem. The no-payload tags ([`FastReload`](AttachTag::FastReload) /
    /// [`WhisperBore`](AttachTag::WhisperBore)) read their magnitude from `tuning`; every
    /// other tag reads its own payload. An empty `slots` folds to the identity.
    #[must_use]
    pub fn from_slots(
        slots: &[AttachTag],
        bundle: &mut WeaponBundle,
        tuning: &AttachmentTuning,
    ) -> Self {
        let mut effects = Self::default();
        for slot in slots {
            effects.apply_slot(slot, bundle, *tuning);
        }
        effects
    }

    /// Dispatch ONE attachment slot to its folder-function — the per-tag `match` arm.
    fn apply_slot(
        &mut self,
        slot: &AttachTag,
        bundle: &mut WeaponBundle,
        tuning: AttachmentTuning,
    ) {
        match slot {
            AttachTag::Stable => bundle.stable = super::Stable::new(true),
            AttachTag::Sighted => self.sighted = Some(Scoped::new(true)),
            AttachTag::Silenced => self.silenced = Some(Silenced::new(true)),
            AttachTag::FastReload => {
                scale_reload(&mut bundle.magazine, tuning.fast_reload_factor);
            }
            AttachTag::HexgrindRounds { punch_bonus } => {
                bundle.punch = add_punch(bundle.punch, *punch_bonus);
            }
            AttachTag::RotgutCoating { damage_type } => bundle.damage_type = *damage_type,
            AttachTag::ButchersWeight {
                damage_bonus,
                tu_percent_delta,
            } => {
                bundle.damage = add_damage(bundle.damage, *damage_bonus);
                add_mode_tu_percent(&mut bundle.fire_mode, *tu_percent_delta);
            }
            AttachTag::WhisperBore => {
                // A suppressor bundled with a modest optic: silence the shot AND steady it.
                // The steadying rides the shared SightStability seam via the Scoped tag,
                // but at the whisper-bore's OWN (smaller) tuning bonus — carried as the
                // per-weapon WeaponSightBonus override so it does not read the full-optic
                // tuning leaf.
                self.silenced = Some(Silenced::new(true));
                self.sighted = Some(Scoped::new(true));
                self.sight_bonus = Some(WeaponSightBonus::new(*tuning.whisper_bore_sight));
            }
            AttachTag::SumpSlickAction {
                reload_factor,
                spread_penalty,
            } => {
                scale_reload(&mut bundle.magazine, *reload_factor);
                bundle.base_spread = add_spread(bundle.base_spread, *spread_penalty);
            }
            AttachTag::GoreSumpDrum {
                size_bonus,
                reload_delta,
            } => {
                grow_magazine(&mut bundle.magazine, *size_bonus);
                add_reload(&mut bundle.magazine, *reload_delta);
            }
            AttachTag::DeadmansBrace { brace_bonus } => {
                // A rigid brace steadies the aim via the shared SightStability seam (the
                // Sighted tag), at the brace's OWN authored magnitude — carried as the
                // per-weapon WeaponSightBonus override so the composer reads the payload
                // points, not the full-optic tuning leaf.
                self.sighted = Some(Scoped::new(true));
                self.sight_bonus = Some(WeaponSightBonus::new(**brace_bonus));
            }
            AttachTag::ExecutionersChoke {
                fatal_bias_bonus,
                cone_mult_delta,
            } => {
                bundle.fatal_bias = add_fatal_bias(bundle.fatal_bias, *fatal_bias_bonus);
                widen_modes(&mut bundle.fire_mode, *cone_mult_delta);
            }
        }
    }

    /// The [`Scoped`](super::Scoped) sibling to spawn on the weapon entity, or `None`.
    #[must_use]
    pub const fn sighted(&self) -> Option<Scoped> {
        self.sighted
    }

    /// The [`Silenced`](super::Silenced) sibling to spawn on the weapon entity, or `None`.
    #[must_use]
    pub const fn silenced(&self) -> Option<Silenced> {
        self.silenced
    }

    /// The per-weapon [`WeaponSightBonus`](super::WeaponSightBonus) override to spawn on
    /// the weapon entity, or `None` (the fitted sight, if any, reads the universal tuning
    /// bonus).
    #[must_use]
    pub const fn sight_bonus(&self) -> Option<WeaponSightBonus> {
        self.sight_bonus
    }
}

/// Whether a `shooter`'s **RANGED** weapon carries the GTW-542 [`Silenced`] tag —
/// the shared gate BOTH loud-signal producers ([`apply_suppression`](crate::suppression::apply_suppression)
/// and [`reaction_trigger`](crate::reaction::reaction_trigger)) key off so a silenced shot
/// propagates neither SUPPRESSION nor REACTION/REVEAL.
///
/// Resolves `shooter → Wields → the RANGED weapon entity` (EXCLUDING the melee weapon via
/// the `melee` probe — the `dispatch_fire` resolution, so the gun's tag is read, never a
/// melee weapon's) then probes that weapon entity for [`Silenced`] via `silenced`. A
/// shooter with no resolvable ranged weapon is treated as NOT silenced (`false`) — the
/// producers then fall through to their normal behaviour rather than silently swallowing
/// a shot from an unresolved source. Faction-blind (both factions' silenced shots stay
/// quiet).
///
/// The `silenced` probe is a `Query<(), With<Silenced>>` — a marker-only archetype filter
/// (the cheapest presence probe), disjoint from the wield/melee queries. Framework
/// system-param plumbing throughout (the no-bare-types carve-out).
#[must_use]
pub fn shooter_weapon_silenced(
    shooter: Entity,
    wields: &WieldsQuery,
    melee: &MeleeQuery,
    silenced: &Query<(), With<Silenced>>,
) -> bool {
    wields
        .get(shooter)
        .ok()
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
        .is_some_and(|weapon| silenced.get(weapon).is_ok())
}

// ── Leaf-rewrite folder-function helpers ──────────────────────────────────────────

/// Add a penetration bonus to a weapon's punch (saturating on the `i32` inner via its
/// public ctor — no bare-field write).
fn add_punch(punch: WeaponPunch, bonus: WeaponPunch) -> WeaponPunch {
    WeaponPunch::new(punch.saturating_add(*bonus))
}

/// Add a base-damage bonus to a weapon's damage (saturating on the `i32` inner).
fn add_damage(damage: WeaponDamage, bonus: WeaponDamage) -> WeaponDamage {
    WeaponDamage::new(damage.saturating_add(*bonus))
}

/// Add a fatal-bias bonus to a weapon's fatal bias (an `f32` addend).
fn add_fatal_bias(bias: FatalBias, bonus: FatalBias) -> FatalBias {
    FatalBias::new(*bias + *bonus)
}

/// Add extra base spread (radians) to a weapon's base spread (an `f32` addend — a wider
/// cone).
fn add_spread(spread: super::BaseSpread, penalty: SpreadPenalty) -> super::BaseSpread {
    super::BaseSpread::new(*spread + *penalty)
}

/// Multiply a magazine's reload cost by a factor, rebuilding the [`Magazine`] through its
/// ctor so its loaded count is preserved and its capacity clamp holds (no bare-field
/// write). The factor is applied in `f32` then floored back to the `u8`
/// [`ReloadTu`](crate::magazine::ReloadTu), clamped to `0`.
fn scale_reload(magazine: &mut Magazine, factor: ReloadFactor) {
    let scaled = (f32::from(*magazine.reload_tu()) * *factor).max(0.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the scaled reload cost is clamped non-negative above and a reload TU is a \
                  small u8 count, so the f32 -> u8 floor cannot truncate meaningfully or \
                  sign-flip (the pos_level storey-narrowing precedent)"
    )]
    let tu = scaled as u8;
    *magazine = Magazine::new(*magazine.rounds(), magazine.size(), ReloadTu::new(tu));
}

/// Add a flat reload-time delta to a magazine's reload cost (saturating on the `u8` inner
/// — a slower swap), rebuilding the [`Magazine`] through its ctor.
fn add_reload(magazine: &mut Magazine, delta: ReloadDelta) {
    let tu = magazine.reload_tu().saturating_add(*delta);
    *magazine = Magazine::new(*magazine.rounds(), magazine.size(), ReloadTu::new(tu));
}

/// Grow a magazine's capacity by a size bonus (saturating on the `u16` inner) and refill
/// it to the NEW full capacity, rebuilding through the ctor so the spawn-full invariant
/// holds for the larger drum.
const fn grow_magazine(magazine: &mut Magazine, bonus: MagazineSize) {
    let size = MagazineSize::new(magazine.size().get().saturating_add(bonus.get()));
    *magazine = Magazine::loaded(size, magazine.reload_tu());
}

/// Add a per-mode TU-percent delta to EVERY fire mode (a heavier weapon fires slower),
/// rebuilding the [`FireMode`] through its ctor. Preserves each mode's `AoE`
/// [`HitType`](super::HitType) (GTW-541) via
/// [`with_hit_type`](super::FireModeSpec::with_hit_type). `FireMode` derefs to its mode
/// slice, so `.iter()` reads the modes directly.
fn add_mode_tu_percent(fire_mode: &mut FireMode, delta: ModeTuPercent) {
    let modes = fire_mode
        .iter()
        .map(|mode| {
            super::FireModeSpec::with_hit_type(
                mode.kind,
                mode.cone_mult,
                ModeTuPercent::new(*mode.tu_percent + *delta),
                mode.shots,
                mode.hit_type,
            )
        })
        .collect();
    *fire_mode = FireMode::new(modes);
}

/// Widen EVERY fire mode's cone multiplier by a delta (a wider, deadlier spray),
/// rebuilding the [`FireMode`] through its ctor. Preserves each mode's `AoE`
/// [`HitType`](super::HitType) (GTW-541).
fn widen_modes(fire_mode: &mut FireMode, delta: ConeMultDelta) {
    let modes = fire_mode
        .iter()
        .map(|mode| {
            super::FireModeSpec::with_hit_type(
                mode.kind,
                ModeConeMult::new(*mode.cone_mult + *delta),
                mode.tu_percent,
                mode.shots,
                mode.hit_type,
            )
        })
        .collect();
    *fire_mode = FireMode::new(modes);
}

#[cfg(test)]
mod tests {
    use super::{AttachTag, AttachmentEffects, Scoped, Silenced};
    use crate::{
        magazine::{Magazine, ReloadTu},
        tuning::AttachmentTuning,
        weapon::{
            Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
            Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
            ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
            WeaponShred,
        },
    };

    /// An arbitrary un-attached ranged bundle (NOT shipped magnitudes) the fold mutates.
    fn base_bundle() -> WeaponBundle {
        WeaponBundle::new(
            WeaponName::new("t".to_owned()),
            BaseSpread::new(0.1),
            Accuracy::new(1.0),
            Kickback::new(0.0),
            FatalBias::new(2.0),
            DamageProfile::new(
                WeaponDamage::new(10),
                WeaponPunch::new(4),
                WeaponShred::new(2),
                DamageType::Kinetic,
            ),
            HandlingProfile::new(
                Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)),
                FireMode::new(vec![FireModeSpec::new(
                    ModeKind::Single,
                    ModeConeMult::new(1.0),
                    ModeTuPercent::new(0.2),
                    ModeShots::new(1),
                )]),
                Stable::new(false),
                Shove::new(false),
                Handedness::OneHanded,
            ),
        )
    }

    #[test]
    fn empty_slots_fold_to_the_identity() {
        let mut bundle = base_bundle();
        let before = bundle.clone();
        let effects = AttachmentEffects::from_slots(&[], &mut bundle, &AttachmentTuning::default());
        assert_eq!(
            bundle, before,
            "an empty slot list leaves the bundle byte-identical"
        );
        assert_eq!(
            effects,
            AttachmentEffects::default(),
            "no sibling tags accumulate"
        );
        assert!(effects.sighted().is_none() && effects.silenced().is_none());
    }

    #[test]
    fn named_tags_accumulate_their_sibling_components() {
        let mut bundle = base_bundle();
        let effects = AttachmentEffects::from_slots(
            &[AttachTag::Silenced, AttachTag::Sighted, AttachTag::Stable],
            &mut bundle,
            &AttachmentTuning::default(),
        );
        assert_eq!(
            effects.silenced(),
            Some(Silenced::new(true)),
            "Silenced sibling accumulates"
        );
        assert_eq!(
            effects.sighted(),
            Some(Scoped::new(true)),
            "Scoped sibling accumulates"
        );
        assert_eq!(
            bundle.stable,
            Stable::new(true),
            "Stable rewrites the spawn-side leaf"
        );
    }

    #[test]
    fn fast_reload_lowers_reload_tu_relative_to_baseline() {
        let mut bundle = base_bundle();
        let base = *bundle.magazine.reload_tu();
        let _ = AttachmentEffects::from_slots(
            &[AttachTag::FastReload],
            &mut bundle,
            &AttachmentTuning::default(),
        );
        assert!(
            *bundle.magazine.reload_tu() < base,
            "FastReload lowers reload_tu ({} < {base})",
            *bundle.magazine.reload_tu(),
        );
    }

    #[test]
    fn hexgrind_raises_punch_relative_to_baseline() {
        let mut bundle = base_bundle();
        let base = *bundle.punch;
        let _ = AttachmentEffects::from_slots(
            &[AttachTag::HexgrindRounds {
                punch_bonus: WeaponPunch::new(5),
            }],
            &mut bundle,
            &AttachmentTuning::default(),
        );
        assert!(
            *bundle.punch > base,
            "HexgrindRounds raises punch ({} > {base})",
            *bundle.punch
        );
    }

    #[test]
    fn whisper_bore_silences_and_scopes() {
        let mut bundle = base_bundle();
        let effects = AttachmentEffects::from_slots(
            &[AttachTag::WhisperBore],
            &mut bundle,
            &AttachmentTuning::default(),
        );
        assert_eq!(
            effects.silenced(),
            Some(Silenced::new(true)),
            "WhisperBore silences the weapon",
        );
        assert_eq!(
            effects.sighted(),
            Some(Scoped::new(true)),
            "WhisperBore also steadies (Scoped)"
        );
        assert!(
            effects.sight_bonus().is_some(),
            "WhisperBore carries its own per-weapon sight bonus",
        );
    }
}
