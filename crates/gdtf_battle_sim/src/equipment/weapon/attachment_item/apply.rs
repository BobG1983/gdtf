//! The **effect-isolation architecture** (GTW-549, PHASE 2 — child of GTW-551 → GTW-17):
//! the [`ApplyAttachmentEffect`] trait whose one method IS an effect's behaviour, one
//! CONCEPTUALLY-ISOLATED type per effect impl-ing it, and the delegating
//! [`AttachmentEffect`](super::AttachmentEffect)-enum bridge that constructs and invokes
//! them.
//!
//! ## The mandate (user effect-isolation ruling, 2026-07-02)
//!
//! The GTW-542 model baked each attachment's effect into a per-tag FOLDER-FUNCTION `match`
//! (`AttachmentEffects::apply_slot`), so adding an effect meant editing a central logic
//! branch. GTW-549 SUPERSEDES that: each effect is its OWN isolated type whose
//! [`apply_to_weapon`](ApplyAttachmentEffect::apply_to_weapon) IS its whole behaviour.
//! Adding a new effect = add ONE type here + ONE `impl` + ONE serde variant + ONE
//! mechanical delegation arm — NO central logic `match`, NO folder-fn, NO authoring step
//! that sends you across the codebase.
//!
//! ## The `&mut EntityWorldMut` carve-out
//!
//! Each effect mutates the (already-spawned) weapon ENTITY through an
//! [`EntityWorldMut`] — the sanctioned exclusive-World access the ticket calls out
//! explicitly (NOT a `bevy-traps` #7 violation): the effect runs as a deferred
//! [`EntityCommand`](bevy::ecs::system::EntityCommand) (via the
//! [`attach_to_weapon`](super::AttachToWeaponExt::attach_to_weapon) commands extension),
//! operating on ONE specific entity that exists post-spawn. It only reads / inserts
//! components (never spawns, never touches other entities), so the access is minimal and
//! scoped.
//!
//! ## Correct stat mapping (the GTW-549 fix)
//!
//! Each effect targets the CORRECT stat: [`ApplyAim`] raises the
//! [`Accuracy`](crate::weapon::Accuracy) exponent (the HEADLINE fix — a sight boosts AIM,
//! NOT stability); [`ApplyStability`] inserts a graduated
//! [`WeaponBraceBonus`](super::WeaponBraceBonus) (a NEW clean `::none()`-identity brace
//! seam, NOT the ripped-out sight-stability); [`ApplyGainFireMode`] appends a
//! [`FireModeSpec`](crate::weapon::FireModeSpec); [`ApplyExtraAmmo`] grows the
//! [`Magazine`](crate::magazine::Magazine); [`ApplyFastReload`] scales its reload cost;
//! [`ApplySilence`] fits [`Silenced`](crate::weapon::Silenced); and so on.

use bevy::prelude::EntityWorldMut;

use super::magnitude::{AimDelta, ReloadScale, WeaponBraceBonus};
use crate::{
    magazine::{Magazine, ReloadTu},
    weapon::{
        Accuracy, DamageType, FatalBias, FireMode, FireModeSpec, MagazineSize, Shove, Silenced,
        Stable, WeaponDamage, WeaponPunch, WeaponShred,
    },
};

/// One attachment effect's **isolated behaviour** — the trait each concrete effect type
/// impls, its single method BEING the effect (GTW-549 PHASE 2, the effect-isolation
/// architecture / user ruling 2026-07-02).
///
/// The whole point: an effect's logic lives in exactly ONE place — its own type's
/// [`apply_to_weapon`](Self::apply_to_weapon) — not in a central `match`, a folder-fn, or a
/// scatter of authoring steps. The [`AttachmentEffect`](super::AttachmentEffect) serde enum
/// impls this by DELEGATING each variant to its isolated type, and the
/// [`attach_to_weapon`](super::AttachToWeaponExt::attach_to_weapon) commands extension
/// invokes it as a deferred [`EntityCommand`](bevy::ecs::system::EntityCommand) against the
/// weapon entity.
pub trait ApplyAttachmentEffect {
    /// Apply this effect to the (already-spawned) `weapon` entity — its whole behaviour.
    ///
    /// Runs inside a deferred [`EntityCommand`](bevy::ecs::system::EntityCommand) with
    /// exclusive access to the ONE weapon entity (the ticket's sanctioned
    /// [`EntityWorldMut`] carve-out). An implementation reads / inserts / rebuilds the
    /// weapon's own components ONLY; it never spawns or touches another entity. When the
    /// target component is absent (a mis-seeded weapon) the effect is a NO-OP rather than a
    /// panic (fail-safe, the `let … else` / `if let` guards below).
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>);
}

/// **Aim** — raises the weapon's [`Accuracy`](crate::weapon::Accuracy) exponent by the
/// per-item [`AimDelta`] (GTW-549 HEADLINE FIX: a sight boosts AIM — clustering the §1b
/// in-cone draw toward centre — NOT stability, which would narrow the cone).
///
/// Additive: reads the existing [`Accuracy`](crate::weapon::Accuracy) and re-inserts it
/// raised by the delta. A weapon with no `Accuracy` (a mis-seeded entity) is left
/// unchanged.
pub struct ApplyAim {
    /// The [`Accuracy`](crate::weapon::Accuracy) addend this sight contributes.
    delta: AimDelta,
}

impl ApplyAim {
    /// Build the aim effect from its per-item [`AimDelta`].
    #[must_use]
    pub const fn new(delta: AimDelta) -> Self {
        Self { delta }
    }
}

impl ApplyAttachmentEffect for ApplyAim {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(accuracy) = weapon.get::<Accuracy>() else {
            return;
        };
        let raised = Accuracy::new(**accuracy + *self.delta);
        weapon.insert(raised);
    }
}

/// **Stability** — fits the weapon a graduated per-item
/// [`WeaponBraceBonus`](super::WeaponBraceBonus) of §1a stability-score points (the NEW
/// clean brace seam GTW-549 introduces, `::none()`-identity — NOT the ripped-out
/// sight-stability seam). GRADUATED, distinct from the boolean [`ApplyBrace`] tag.
///
/// Inserts the bonus as a `Component` (the §1a stability composer folds it as an additive
/// contribution, the [`SuppressionStability`](crate::SuppressionStability) precedent).
pub struct ApplyStability {
    /// The graduated §1a stability-score points this brace contributes.
    bonus: WeaponBraceBonus,
}

impl ApplyStability {
    /// Build the stability effect from its per-item [`WeaponBraceBonus`](super::WeaponBraceBonus).
    #[must_use]
    pub const fn new(bonus: WeaponBraceBonus) -> Self {
        Self { bonus }
    }
}

impl ApplyAttachmentEffect for ApplyStability {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(self.bonus);
    }
}

/// **`GainFireMode`** — ADDS one [`FireModeSpec`](crate::weapon::FireModeSpec) to the
/// weapon's [`FireMode`](crate::weapon::FireMode) selector (a conversion kit granting a new
/// firing mode).
///
/// Reads the existing selector, appends the new mode, and re-inserts the rebuilt
/// [`FireMode`](crate::weapon::FireMode). A weapon with no selector is left unchanged.
pub struct ApplyGainFireMode {
    /// The fire mode this conversion kit adds to the selector.
    mode: FireModeSpec,
}

impl ApplyGainFireMode {
    /// Build the gain-fire-mode effect from the [`FireModeSpec`](crate::weapon::FireModeSpec)
    /// it grants.
    #[must_use]
    pub const fn new(mode: FireModeSpec) -> Self {
        Self { mode }
    }
}

impl ApplyAttachmentEffect for ApplyGainFireMode {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(fire_mode) = weapon.get::<FireMode>() else {
            return;
        };
        let mut modes: Vec<FireModeSpec> = fire_mode.iter().copied().collect();
        modes.push(self.mode);
        weapon.insert(FireMode::new(modes));
    }
}

/// **`ExtraAmmo`** — GROWS the weapon's [`Magazine`](crate::magazine::Magazine) capacity by
/// its per-item [`MagazineSize`](crate::weapon::MagazineSize) and refills it to the new full
/// (an oversized drum, more rounds before a reload).
///
/// Rebuilds the [`Magazine`](crate::magazine::Magazine) through its ctor (the GTW-542
/// `grow_magazine` logic, now isolated here) so the spawn-full invariant holds for the
/// larger drum. A weapon with no magazine is left unchanged.
pub struct ApplyExtraAmmo {
    /// The extra capacity this drum adds to the magazine.
    size_bonus: MagazineSize,
}

impl ApplyExtraAmmo {
    /// Build the extra-ammo effect from the [`MagazineSize`](crate::weapon::MagazineSize)
    /// capacity it adds.
    #[must_use]
    pub const fn new(size_bonus: MagazineSize) -> Self {
        Self { size_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyExtraAmmo {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(magazine) = weapon.get::<Magazine>() else {
            return;
        };
        let grown = MagazineSize::new(magazine.size().get().saturating_add(self.size_bonus.get()));
        let reload_tu = magazine.reload_tu();
        weapon.insert(Magazine::loaded(grown, reload_tu));
    }
}

/// **`FastReload`** — SCALES the weapon's [`ReloadTu`](crate::magazine::ReloadTu) reload cost
/// by its per-item [`ReloadScale`] (`< 1.0` → a faster reload; GTW-549 re-homes the magnitude
/// onto the item, the exact defect the GTW-542 global-tuning model had).
///
/// Rebuilds the [`Magazine`](crate::magazine::Magazine) through its ctor preserving the
/// loaded count + capacity (the GTW-542 `scale_reload` logic, now isolated here). A weapon
/// with no magazine is left unchanged.
pub struct ApplyFastReload {
    /// The per-item reload-cost multiplier (`< 1.0` speeds the reload).
    scale: ReloadScale,
}

impl ApplyFastReload {
    /// Build the fast-reload effect from its per-item [`ReloadScale`].
    #[must_use]
    pub const fn new(scale: ReloadScale) -> Self {
        Self { scale }
    }
}

impl ApplyAttachmentEffect for ApplyFastReload {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(magazine) = weapon.get::<Magazine>() else {
            return;
        };
        let scaled = (f32::from(*magazine.reload_tu()) * *self.scale).max(0.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the scaled reload cost is clamped non-negative above and a reload TU is a \
                      small u8 count, so the f32 -> u8 floor cannot truncate meaningfully or \
                      sign-flip (the GTW-542 scale_reload precedent this isolates)"
        )]
        let tu = scaled as u8;
        let rebuilt = Magazine::new(*magazine.rounds(), magazine.size(), ReloadTu::new(tu));
        weapon.insert(rebuilt);
    }
}

/// **Silence** — fits the [`Silenced`](crate::weapon::Silenced)`(true)` tag so the weapon's
/// shots propagate neither SUPPRESSION nor REACTION/REVEAL (a suppressor). PRESERVES the
/// GTW-542 [`Silenced`](crate::weapon::Silenced) component + BOTH its producer gates
/// unchanged — this effect only INSERTS the tag those gates already read.
///
/// A no-payload unit effect (the tag carries its own meaning).
pub struct ApplySilence;

impl ApplyAttachmentEffect for ApplySilence {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(Silenced::new(true));
    }
}

/// **Penetration** — ADDS its [`WeaponPunch`](crate::weapon::WeaponPunch) to the weapon's
/// penetration (armour-piercing rounds, ignores more armour).
///
/// Additive (saturating on the `i32` inner via the public ctor). A weapon with no punch is
/// left unchanged.
pub struct ApplyPenetration {
    /// The extra penetration these rounds add.
    punch_bonus: WeaponPunch,
}

impl ApplyPenetration {
    /// Build the penetration effect from the [`WeaponPunch`](crate::weapon::WeaponPunch) it
    /// adds.
    #[must_use]
    pub const fn new(punch_bonus: WeaponPunch) -> Self {
        Self { punch_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyPenetration {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(punch) = weapon.get::<WeaponPunch>() else {
            return;
        };
        let raised = WeaponPunch::new(punch.saturating_add(*self.punch_bonus));
        weapon.insert(raised);
    }
}

/// **`DamageTypeOverride`** — OVERRIDES the weapon's emitted
/// [`DamageType`](crate::weapon::DamageType) (a toxic / elemental coating, a matchup-wheel
/// re-key).
///
/// A full override (inserts the new [`DamageType`](crate::weapon::DamageType) regardless of
/// the prior value — an insert replaces the component).
pub struct ApplyDamageTypeOverride {
    /// The damage type this coating forces the weapon to emit.
    damage_type: DamageType,
}

impl ApplyDamageTypeOverride {
    /// Build the damage-type-override effect from the [`DamageType`](crate::weapon::DamageType)
    /// it forces.
    #[must_use]
    pub const fn new(damage_type: DamageType) -> Self {
        Self { damage_type }
    }
}

impl ApplyAttachmentEffect for ApplyDamageTypeOverride {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(self.damage_type);
    }
}

/// **Damage** — ADDS its [`WeaponDamage`](crate::weapon::WeaponDamage) to the weapon's base
/// damage (a brutal counterweight / hotter load).
///
/// Additive (saturating on the `i32` inner). A weapon with no damage stat is left unchanged.
/// USER-REVIEW extra (defensible default).
pub struct ApplyDamage {
    /// The extra base damage this attachment adds.
    damage_bonus: WeaponDamage,
}

impl ApplyDamage {
    /// Build the damage effect from the [`WeaponDamage`](crate::weapon::WeaponDamage) it adds.
    #[must_use]
    pub const fn new(damage_bonus: WeaponDamage) -> Self {
        Self { damage_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyDamage {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(damage) = weapon.get::<WeaponDamage>() else {
            return;
        };
        let raised = WeaponDamage::new(damage.saturating_add(*self.damage_bonus));
        weapon.insert(raised);
    }
}

/// **Shred** — ADDS its [`WeaponShred`](crate::weapon::WeaponShred) to the weapon's
/// armour-durability damage (a serrated attachment).
///
/// Additive (saturating on the `i32` inner). A weapon with no shred stat is left unchanged.
/// USER-REVIEW extra (defensible default).
pub struct ApplyShred {
    /// The extra integrity damage this attachment adds.
    shred_bonus: WeaponShred,
}

impl ApplyShred {
    /// Build the shred effect from the [`WeaponShred`](crate::weapon::WeaponShred) it adds.
    #[must_use]
    pub const fn new(shred_bonus: WeaponShred) -> Self {
        Self { shred_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyShred {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(shred) = weapon.get::<WeaponShred>() else {
            return;
        };
        let raised = WeaponShred::new(shred.saturating_add(*self.shred_bonus));
        weapon.insert(raised);
    }
}

/// **`FatalBias`** — ADDS its [`FatalBias`](crate::weapon::FatalBias) to the weapon's
/// severity-score addend (a savage muzzle, nastier §6 wound buckets).
///
/// Additive (`f32` addend). A weapon with no fatal-bias stat is left unchanged.
/// USER-REVIEW extra (defensible default).
pub struct ApplyFatalBias {
    /// The extra fatal-bias this attachment adds.
    bias_bonus: FatalBias,
}

impl ApplyFatalBias {
    /// Build the fatal-bias effect from the [`FatalBias`](crate::weapon::FatalBias) it adds.
    #[must_use]
    pub const fn new(bias_bonus: FatalBias) -> Self {
        Self { bias_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyFatalBias {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(bias) = weapon.get::<FatalBias>() else {
            return;
        };
        let raised = FatalBias::new(**bias + *self.bias_bonus);
        weapon.insert(raised);
    }
}

/// **Brace** — fits the [`Stable`](crate::weapon::Stable)`(true)` tag (the §1a UNCONDITIONAL
/// brace). DISTINCT from the graduated [`ApplyStability`] — a boolean tag, not a magnitude.
///
/// A no-payload unit effect. USER-REVIEW extra (defensible default).
pub struct ApplyBrace;

impl ApplyAttachmentEffect for ApplyBrace {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(Stable::new(true));
    }
}

/// **Shove** — fits the [`Shove`](crate::weapon::Shove)`(true)` tag (GTW-525 — knocks the
/// target back one cell on a connecting hit).
///
/// A no-payload unit effect. USER-REVIEW extra (defensible default).
pub struct ApplyShove;

impl ApplyAttachmentEffect for ApplyShove {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(Shove::new(true));
    }
}
