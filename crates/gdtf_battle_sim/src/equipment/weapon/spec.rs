//! The **authoring spec** — the `WeaponSpec` an `assets/content/weapons/ranged/*.weapon.ron`
//! deserializes into, plus [`into_bundle`](WeaponSpec::into_bundle) which resolves
//! it into a spawnable [`WeaponBundle`] (GTW-257).

use bevy::reflect::TypePath;
use serde::Deserialize;

use super::{
    Accuracy, AttachTag, AttachmentEffects, BaseSpread, DamageProfile, DamageType, DotProfile,
    FatalBias, FireMode, Handedness, HandlingProfile, Kickback, Shove, Stable, WeaponBundle,
    WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
};
use crate::{magazine::Magazine, tuning::AttachmentTuning};

/// The **authoring struct** an `assets/content/weapons/ranged/*.ron` deserializes into — every
/// weapon NUMBER the §1/§6 math reads, MINUS the [`WeaponName`] (the name is the
/// FILE KEY, supplied by the loader from the file's stem) and MINUS the
/// [`Weapon`](super::Weapon) marker (that is added by [`WeaponBundle::new`]).
///
/// This is the data-driven, folder-loaded weapon model (the
/// [[weapons-armor-data-driven]] end-state, GTW-257): a per-weapon loose `.ron`
/// file is parsed into a `WeaponSpec`, keyed by its filename stem into the
/// [`WeaponRegistry`](super::WeaponRegistry), and resolved at battle setup into a
/// [`WeaponBundle`] via [`into_bundle`](WeaponSpec::into_bundle). It mirrors
/// [`WeaponBundle`]'s data exactly, dropping only the two fields the loader /
/// spawn-side own: the name (the file key) and the marker (the armed-entity tag).
///
/// Every field is an existing weapon-number newtype authored as its
/// `#[serde(transparent)]` bare RON scalar (the [`crate::tuning`] / GTW-200 house
/// style); the authored magnitudes are tuning DATA (commented in the `.ron`), NOT
/// pinned by tests (the brittle-test rule). Derives [`Deserialize`] so the loose
/// `.ron` parses, and [`TypePath`] because the `RonAsset<WeaponSpec>` the loader wraps
/// it in requires its payload to be [`TypePath`] (the same bound
/// [`Situation`](crate::lifecycle::situation::Situation) /
/// [`CombatTuning`](crate::tuning::CombatTuning) satisfy).
///
/// **Not `Copy`** — it owns a [`FireMode`] (which holds a `Vec` of specs); it is
/// `Clone`, so the registry can hold specs BY VALUE.
#[derive(Debug, Clone, PartialEq, Deserialize, TypePath)]
pub struct WeaponSpec {
    /// The intrinsic angular spread before situational multipliers (`base_spread`).
    pub base_spread:      BaseSpread,
    /// The concentration weapon term (`accuracy`; may exceed 1.0).
    pub accuracy:         Accuracy,
    /// The per-round recoil added in a burst (`kickback`).
    pub kickback:         Kickback,
    /// The severity-score addend, consumed by E3 (`fatal_bias`).
    pub fatal_bias:       FatalBias,
    /// The base damage a hit deals before armor (`damage`).
    pub damage:           WeaponDamage,
    /// The armor protection a hit ignores — penetration (`punch`).
    pub punch:            WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability (`shred`).
    pub shred:            WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type:      DamageType,
    /// The ammo state — the [`Magazine`] grouping authored as `(size: N, reload_tu: N)`
    /// (the per-weapon round capacity + reload TU cost). The live loaded-rounds count
    /// is NOT authored (it defaults to `0` on deserialize); [`into_bundle`](WeaponSpec::into_bundle)
    /// spawns the magazine FULL (`loaded == size`), the GTW-275 spawn-full path.
    pub magazine:         Magazine,
    /// The authored fire-mode selector — the list of offered modes, each a
    /// [`FireModeSpec`](super::FireModeSpec) carrying its [`ModeKind`](super::ModeKind)
    /// + cone/TU%/shots.
    pub fire_mode:        FireMode,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally.
    pub stable:           Stable,
    /// The `shove` tag (GTW-525) — `true` knocks the target back one cell on a
    /// connecting shot (in addition to the shot's damage). `#[serde(default)]` so an
    /// omitted `shove:` field falls back to `Shove(false)` (a non-shove weapon): the
    /// tag is OPT-IN, so the many existing weapon `.ron`s that never author it keep
    /// shoving OFF (the [`Reach`](super::Reach) `#[serde(default)]` precedent), unlike
    /// the required `stable:` field.
    #[serde(default)]
    pub shove:            Shove,
    /// The weapon's [`Handedness`] (GTW-443) — `OneHanded` (a pistol) or `TwoHanded`
    /// (a long-arm / heavy piece); authored as the `handedness:` field of the
    /// `.weapon.ron`. The shared `can_fire` guard refuses a `TwoHanded` weapon below two
    /// available hands.
    pub handedness:       Handedness,
    /// The weapon's fitted **attachments** (GTW-542, child GTW-41b) — the list of
    /// [`AttachTag`]s the weapon carries, authored as the `attachment_slots:` `.weapon.ron`
    /// field. `#[serde(default)]` so an omitted field falls back to an EMPTY list (a weapon
    /// with no attachments): the field is OPT-IN (the [`Shove`] `#[serde(default)]`
    /// precedent), so EVERY existing weapon `.ron` — none of which author it — deserializes
    /// and spawns BYTE-IDENTICAL. [`into_bundle`](WeaponSpec::into_bundle) FOLDS this list
    /// through the per-tag folder-functions ([`AttachmentEffects`](super::AttachmentEffects));
    /// the empty default folds to the identity (no leaf change, no sibling tags).
    #[serde(default)]
    pub attachment_slots: Vec<AttachTag>,
    /// The weapon's optional **damage-over-time profile** (GTW-544, child GTW-41e) — the
    /// `{ damage, DamageType, turns }` a DOT weapon (a chem sprayer, a plasma torch) carries,
    /// authored as the `dot:` `.weapon.ron` field. `#[serde(default)]` (defaulting to `None`)
    /// so an omitted field is a NON-DOT weapon: the field is OPT-IN (the `attachment_slots` /
    /// [`Shove`] `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of
    /// which author it — deserializes and spawns BYTE-IDENTICAL. When present,
    /// [`into_bundle`](WeaponSpec::into_bundle) carries it into the resolved
    /// [`AttachmentEffects`](super::AttachmentEffects) as the `dot` sibling the wielded-weapon
    /// scene seam composes onto the weapon entity, so a penetrating hit from this weapon
    /// attaches a [`Dot`](super::Dot) on the struck ganger.
    #[serde(default)]
    pub dot:              Option<DotProfile>,
    /// The weapon's optional **on-death effect** (GTW-547, child GTW-41g) — the
    /// [`OnDeathEffect`](crate::on_death::OnDeathEffect) (`Explode` / `LeaveField`) the WIELDING
    /// ganger's death fans (a live grenade, an unstable power cell), authored as the `on_death:`
    /// `.weapon.ron` field. `#[serde(default)]` (defaulting to `None`) so an omitted field is a
    /// weapon with no death effect: the field is OPT-IN (the `dot` / `attachment_slots` /
    /// [`Shove`] `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of which
    /// author it — deserializes and spawns BYTE-IDENTICAL. When present,
    /// [`into_bundle`](WeaponSpec::into_bundle) carries it into the resolved
    /// [`AttachmentEffects`](super::AttachmentEffects) as the `on_death` sibling the
    /// wielded-weapon scene seam composes onto the weapon entity (as an
    /// [`OnDeath`](crate::on_death::OnDeath) component), so
    /// [`resolve_on_death`](crate::on_death::resolve_on_death) fans it when the ganger dies.
    #[serde(default)]
    pub on_death:         Option<crate::on_death::OnDeathEffect>,
}

impl WeaponSpec {
    /// Resolve this authored spec into a spawnable [`WeaponBundle`] plus its resolved
    /// GTW-542 [`AttachmentEffects`], supplying the [`WeaponName`] from the registry KEY
    /// (the weapon file's filename stem) and the [`AttachmentTuning`] the no-payload
    /// attachment tags read.
    ///
    /// Groups the per-hit damage fields into a [`DamageProfile`] and the
    /// magazine/fire-mode/`stable`/`shove` fields into a [`HandlingProfile`], calls
    /// [`WeaponBundle::new`] (the [`Weapon`](super::Weapon) marker is added there), then
    /// FOLDS this spec's [`attachment_slots`](WeaponSpec::attachment_slots) through the
    /// per-tag folder-functions ([`AttachmentEffects::from_slots`], GTW-542): each slot
    /// either rewrites a spawn-side LEAF on the bundle (damage / punch / magazine /
    /// fire-mode / … ) or produces a SIBLING tag ([`Scoped`](super::Scoped) /
    /// [`Silenced`](super::Silenced) / [`WeaponSightBonus`](super::WeaponSightBonus)) the
    /// caller spawns onto the weapon entity. An EMPTY slot list folds to the identity, so a
    /// weapon with no attachments resolves BYTE-IDENTICAL to before GTW-542.
    ///
    /// The spawned [`Magazine`] is built FULL (loaded to `size`) from the authored `size` +
    /// `reload_tu` (the GTW-275 "full magazine at spawn" path), before any attachment
    /// reload-time / capacity rewrite. Consumes the spec by value (it owns the
    /// [`FireMode`] and the slot list); a caller holding a borrowed spec clones it first
    /// (the registry's specs are `Clone`).
    #[must_use]
    pub fn into_bundle(
        self,
        name: WeaponName,
        attachment_tuning: &AttachmentTuning,
    ) -> (WeaponBundle, AttachmentEffects) {
        let magazine = Magazine::loaded(self.magazine.size(), self.magazine.reload_tu());
        let mut bundle = WeaponBundle::new(
            name,
            self.base_spread,
            self.accuracy,
            self.kickback,
            self.fatal_bias,
            DamageProfile::new(self.damage, self.punch, self.shred, self.damage_type),
            HandlingProfile::new(
                magazine,
                self.fire_mode,
                self.stable,
                self.shove,
                self.handedness,
            ),
        );
        // GTW-542: fold the attachment slots — mutating the bundle's leaves in place and
        // collecting the sibling tags the spawn seam composes onto the weapon entity.
        // GTW-544: carry the authored `dot` profile into the SAME effects accumulator (as its
        // `dot` sibling), AFTER the slot fold — a DOT profile is a spawn-side sibling component
        // the wielded-weapon scene seam composes exactly like the Scoped / Silenced tags. A
        // `None` (a non-DOT weapon) leaves the accumulator identity, so the weapon spawns
        // byte-identical to before this slice.
        // GTW-547: carry the authored `on_death` effect into the SAME effects accumulator (its
        // `on_death` sibling), so the wielded-weapon scene seam composes an `OnDeath` component
        // exactly like the `dot` / attachment siblings. A `None` leaves the accumulator
        // identity, so a weapon with no death effect spawns byte-identical.
        let effects =
            AttachmentEffects::from_slots(&self.attachment_slots, &mut bundle, attachment_tuning)
                .with_dot(self.dot)
                .with_on_death(self.on_death);
        (bundle, effects)
    }
}
