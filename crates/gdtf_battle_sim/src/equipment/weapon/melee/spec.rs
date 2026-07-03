//! The melee **authoring spec** — the [`MeleeWeaponSpec`] an
//! `assets/content/weapons/melee/*.melee_weapon.ron` deserializes into, plus
//! [`into_bundle`](MeleeWeaponSpec::into_bundle) which resolves it into a spawnable
//! [`MeleeWeaponBundle`] (GTW-505, child GTW-37a of the GTW-37 melee epic).

use bevy::reflect::TypePath;
use serde::Deserialize;

use super::{FightMode, MeleeDamageProfile, MeleeWeaponBundle, Reach};
use crate::weapon::{
    AttachmentName, DamageType, FatalBias, Handedness, Shove, WeaponDamage, WeaponName,
    WeaponPunch, WeaponShred, WeaponSlots,
};

/// The **authoring struct** an `assets/content/weapons/melee/*.melee_weapon.ron`
/// deserializes into — the melee mirror of the ranged
/// [`WeaponSpec`](super::super::WeaponSpec), MINUS the [`WeaponName`] (the name is the
/// FILE KEY, supplied by the loader from the file's stem) and MINUS the
/// [`MeleeWeapon`](super::MeleeWeapon) marker (added by [`MeleeWeaponBundle::new`]).
///
/// It SHARES the ranged damage model verbatim (the [`WeaponDamage`] / [`WeaponPunch`] /
/// [`WeaponShred`] / [`DamageType`] damage group + the [`FatalBias`] + the
/// [`Handedness`]) and adds the melee-only fields — the [`Reach`] and the
/// [`FightMode`] selector — plus the shared GTW-525 [`Shove`] knockback tag, DROPPING
/// every ranged-only cone/handling field (`base_spread` / `accuracy` / `kickback` /
/// `magazine` / `stable`). GTW-554 adds FULL attachment support — the [`slots`](Self::slots)
/// declaration + the [`attachments`](Self::attachments) key list, the ranged mirror —
/// resolved at setup through the same slot-gated seam (not folded in
/// [`into_bundle`](Self::into_bundle)).
///
/// Every shared field is its existing weapon-number newtype authored as its
/// `#[serde(transparent)]` bare RON scalar (the [`crate::tuning`] / GTW-200 house
/// style); the authored magnitudes are tuning DATA (commented in the `.ron`), NOT
/// pinned by tests (the brittle-test rule). The [`Reach`] field is
/// `#[serde(default)]` — an omitted `reach:` falls back to [`Reach::DEFAULT`] (`1`), so
/// the first-slice "Reach default 1" ruling holds for a weapon that does not author it.
/// Derives [`Deserialize`] so the loose `.ron` parses, and [`TypePath`] because the
/// `RonAsset<MeleeWeaponSpec>` the loader wraps it in requires its payload to be
/// [`TypePath`] (the same bound [`WeaponSpec`](super::super::WeaponSpec) satisfies).
///
/// **Not `Copy`** — it owns a [`FightMode`] (which holds a `Vec`); it is `Clone`, so
/// the registry can hold specs BY VALUE.
#[derive(Debug, Clone, PartialEq, Deserialize, TypePath)]
pub struct MeleeWeaponSpec {
    /// The base damage a strike deals before armor (`damage`) — the shared ranged newtype.
    pub damage:      WeaponDamage,
    /// The armor protection a strike ignores — penetration (`punch`) — the shared ranged newtype.
    pub punch:       WeaponPunch,
    /// The extra integrity damage a strike deals to armor durability (`shred`) — the shared
    /// ranged newtype.
    pub shred:       WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node — the shared ranged newtype.
    pub damage_type: DamageType,
    /// The severity-score addend, consumed by E3 (`fatal_bias`) — the shared ranged newtype.
    pub fatal_bias:  FatalBias,
    /// The weapon's [`Handedness`] (`handedness`) — the shared ranged newtype.
    pub handedness:  Handedness,
    /// The melee-only [`Reach`] — how many cells away a strike can land (GTW-505).
    /// `#[serde(default)]` so an omitted field falls back to [`Reach::DEFAULT`] (`1`).
    #[serde(default)]
    pub reach:       Reach,
    /// The melee-only authored [`FightMode`] selector — the list of offered fight modes,
    /// each a [`FightModeSpec`](super::FightModeSpec) carrying its
    /// [`FightModeKind`](super::FightModeKind) + flat TU cost + strike count.
    pub fight_mode:  FightMode,
    /// The `shove` tag (GTW-525) — `true` knocks the target back one cell on a
    /// connecting melee strike. `#[serde(default)]` so an omitted `shove:` field falls
    /// back to `Shove(false)` (a non-shove weapon): the tag is OPT-IN (the [`Reach`]
    /// `#[serde(default)]` precedent), so a melee weapon that never authors it keeps
    /// knockback OFF.
    #[serde(default)]
    pub shove:       Shove,
    /// The melee weapon's declared **attachment slots** (GTW-554 — melee weapons gain FULL
    /// attachment support): the [`WeaponSlots`] pair list authored as the `slots:`
    /// `.melee_weapon.ron` field, e.g. `slots: [(Counterweight, 1), (Pommel, 1)]`.
    /// `#[serde(default)]` — an omitted field is the EMPTY declaration (bare fists take no
    /// fittings; fail-closed). Class gating EMERGES from the declarations: a melee weapon
    /// simply never declares `Muzzle`/`Sight`/`Rail`, so ranged-style items find no slot.
    #[serde(default)]
    pub slots:       WeaponSlots,
    /// The melee weapon's fitted **attachments** (GTW-554) — the [`AttachmentName`] KEYS it
    /// references, the exact ranged
    /// [`attachments`](crate::weapon::WeaponSpec::attachments) mirror. `#[serde(default)]`
    /// (an omitted field fits nothing). At battle setup the keys resolve through the SAME
    /// slot-gated [`resolve_pending_attachments`](crate::weapon::resolve_pending_attachments)
    /// seam as the ranged path and ride onto the spawned MELEE weapon entity as a
    /// [`PendingAttachments`](crate::weapon::PendingAttachments) marker the post-spawn
    /// applier consumes.
    #[serde(default)]
    pub attachments: Vec<AttachmentName>,
}

impl MeleeWeaponSpec {
    /// Resolve this authored melee spec into a spawnable [`MeleeWeaponBundle`],
    /// supplying the [`WeaponName`] from the registry KEY (the file's filename stem).
    ///
    /// Groups the per-hit damage fields into a [`MeleeDamageProfile`], then calls
    /// [`MeleeWeaponBundle::new`] — the [`MeleeWeapon`](super::MeleeWeapon) marker is
    /// added there. Consumes the spec by value (it owns the [`FightMode`]); a caller
    /// holding a borrowed spec clones it first (the registry's specs are `Clone`), the
    /// ranged [`WeaponSpec::into_bundle`](super::super::WeaponSpec::into_bundle)
    /// precedent.
    #[must_use]
    pub fn into_bundle(self, name: WeaponName) -> MeleeWeaponBundle {
        MeleeWeaponBundle::new(
            name,
            MeleeDamageProfile::new(self.damage, self.punch, self.shred, self.damage_type),
            self.fatal_bias,
            self.handedness,
            self.reach,
            self.fight_mode,
            self.shove,
        )
    }
}
