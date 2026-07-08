//! The **authoring spec** — the `WeaponSpec` an `assets/content/weapons/ranged/*.weapon.ron`
//! deserializes into, plus [`into_bundle`](WeaponSpec::into_bundle) which resolves
//! it into a spawnable [`WeaponBundle`] (GTW-257), and the two spawn-side sibling records
//! ([`WeaponSpawnSiblings`] / the [`PendingAttachments`] component) the wielded-weapon scene
//! composes.

use bevy::{prelude::Component, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{
    Accuracy, BaseSpread, DamageProfile, DamageType, DotProfile, FatalBias, FireMode, Handedness,
    HandlingProfile, Kickback, Shove, Stable, TrajectoryStyle, WeaponBundle, WeaponDamage,
    WeaponName, WeaponPunch, WeaponShred,
};
use crate::{
    effects::attachments::AttachmentEffect,
    equipment::attachments::{AttachmentName, WeaponSlots},
    magazine::Magazine,
};

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
///
/// [`Serialize`] too (GTW-670) so the editor's WEAPON mode saves a `.weapon.ron` in the
/// SAME schema it loads (the round-trip contract — never a parallel serialize-only
/// mirror); the [`Magazine`]'s live `rounds` count is `#[serde(skip_serializing)]` on its
/// own field, so a saved file authors only `(size, reload_tu)` exactly like every
/// hand-authored member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypePath)]
pub struct WeaponSpec {
    /// The intrinsic angular spread before situational multipliers (`base_spread`).
    pub base_spread: BaseSpread,
    /// The concentration weapon term (`accuracy`; may exceed 1.0).
    pub accuracy:    Accuracy,
    /// The per-round recoil added in a burst (`kickback`).
    pub kickback:    Kickback,
    /// The severity-score addend, consumed by E3 (`fatal_bias`).
    pub fatal_bias:  FatalBias,
    /// The base damage a hit deals before armor (`damage`).
    pub damage:      WeaponDamage,
    /// The armor protection a hit ignores — penetration (`punch`).
    pub punch:       WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability (`shred`).
    pub shred:       WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type: DamageType,
    /// The ammo state — the [`Magazine`] grouping authored as `(size: N, reload_tu: N)`
    /// (the per-weapon round capacity + reload TU cost). The live loaded-rounds count
    /// is NOT authored (it defaults to `0` on deserialize); [`into_bundle`](WeaponSpec::into_bundle)
    /// spawns the magazine FULL (`loaded == size`), the GTW-275 spawn-full path.
    pub magazine:    Magazine,
    /// The authored fire-mode selector — the list of offered modes, each a
    /// [`FireModeSpec`](super::FireModeSpec) carrying its [`ModeKind`](super::ModeKind)
    /// + cone/TU%/shots.
    pub fire_mode:   FireMode,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally.
    pub stable:      Stable,
    /// The `shove` tag (GTW-525) — `true` knocks the target back one cell on a
    /// connecting shot (in addition to the shot's damage). `#[serde(default)]` so an
    /// omitted `shove:` field falls back to `Shove(false)` (a non-shove weapon): the
    /// tag is OPT-IN, so the many existing weapon `.ron`s that never author it keep
    /// shoving OFF (the [`Reach`](super::Reach) `#[serde(default)]` precedent), unlike
    /// the required `stable:` field.
    #[serde(default)]
    pub shove:       Shove,
    /// The weapon's [`Handedness`] (GTW-443) — `OneHanded` (a pistol) or `TwoHanded`
    /// (a long-arm / heavy piece); authored as the `handedness:` field of the
    /// `.weapon.ron`. The shared `can_fire` guard refuses a `TwoHanded` weapon below two
    /// available hands.
    pub handedness:  Handedness,
    /// The weapon's **trajectory style** (GTW-546, child GTW-41d) — `Straight` (a flat ray)
    /// or `Arc` (a lobbed grenade parabola), authored as the `trajectory:` `.weapon.ron`
    /// field. `#[serde(default)]` (defaulting to [`TrajectoryStyle::Straight`]) so an omitted
    /// field is a flat-firing weapon: the field is OPT-IN (the `shove` / `attachments`
    /// `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of which author
    /// it — deserializes and spawns BYTE-IDENTICAL. A grenade / grenade launcher authors
    /// `trajectory: Arc`, spawning a [`TrajectoryStyle::Arc`] component the throw path reads to
    /// lob the round via [`march_arc`](crate::march::march_arc) with no LOS gate.
    #[serde(default)]
    pub trajectory:  TrajectoryStyle,
    /// The weapon's declared **attachment slots** (GTW-554) — the
    /// [`WeaponSlots`] pair list authored as the `slots:` `.weapon.ron` field, e.g.
    /// `slots: [(Muzzle, 1), (Sight, 1), (Rail, 3)]`: WHICH
    /// [`AttachmentSlot`](crate::equipment::attachments::AttachmentSlot)s this weapon offers and how many
    /// attachments each holds. `#[serde(default)]` so an omitted field is the EMPTY
    /// declaration — the weapon offers NO slots, so no attachment fits it (fail-closed; a
    /// thrown grenade authors none). The fit gate
    /// ([`attachment_fits`](crate::equipment::attachments::attachment_fits)) admits each `attachments` key's item
    /// only into a declared slot with free capacity; class gating EMERGES from these
    /// declarations (no ranged/melee tag exists on an item).
    #[serde(default)]
    pub slots:       WeaponSlots,
    /// The weapon's fitted **attachments** (GTW-549, child GTW-551) — the list of
    /// [`AttachmentName`] KEYS the weapon references (authored once in
    /// `assets/content/attachments/*.attachment.ron`, referenced by many weapons), authored as
    /// the `attachments:` `.weapon.ron` field. `#[serde(default)]` so an omitted field falls
    /// back to an EMPTY list (a weapon with no attachments): the field is OPT-IN (the
    /// [`Shove`] `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of which
    /// author it — deserializes and spawns BYTE-IDENTICAL. At battle setup each key is resolved
    /// against the [`AttachmentRegistry`](crate::equipment::attachments::AttachmentRegistry) — GATED by the GTW-554
    /// slot fit ([`resolve_pending_attachments`](crate::equipment::attachments::resolve_pending_attachments)): an item
    /// whose [`slot`](crate::equipment::attachments::AttachmentSpec::slot) is undeclared in [`slots`](Self::slots) or
    /// already at capacity is CLEANLY REJECTED (skipped, never evicted) — and each FITTING
    /// item's [`AttachmentEffect`]s are applied to the spawned weapon entity via the
    /// [`attach_to_weapon`](crate::equipment::attachments::AttachToWeaponExt::attach_to_weapon) commands extension
    /// (SUPERSEDES the GTW-542 `attachment_slots: Vec<AttachTag>` inline-enum model). The empty
    /// default applies NO effects.
    #[serde(default)]
    pub attachments: Vec<AttachmentName>,
    /// The weapon's optional **damage-over-time profile** (GTW-544, child GTW-41e) — the
    /// `{ damage, DamageType, turns }` a DOT weapon (a chem sprayer, a plasma torch) carries,
    /// authored as the `dot:` `.weapon.ron` field. `#[serde(default)]` (defaulting to `None`)
    /// so an omitted field is a NON-DOT weapon: the field is OPT-IN (the `attachments` /
    /// [`Shove`] `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of
    /// which author it — deserializes and spawns BYTE-IDENTICAL. When present,
    /// [`into_bundle`](WeaponSpec::into_bundle) carries it into the resolved
    /// [`WeaponSpawnSiblings`] as the `dot` sibling the wielded-weapon scene seam composes onto
    /// the weapon entity, so a penetrating hit from this weapon attaches a
    /// [`Dot`](super::Dot) on the struck ganger.
    #[serde(default)]
    pub dot:         Option<DotProfile>,
    /// The weapon's optional **on-death effect** (GTW-547, child GTW-41g) — the
    /// [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect) (`Explode` / `LeaveField`) the WIELDING
    /// ganger's death fans (a live grenade, an unstable power cell), authored as the `on_death:`
    /// `.weapon.ron` field. `#[serde(default)]` (defaulting to `None`) so an omitted field is a
    /// weapon with no death effect: the field is OPT-IN (the `dot` / `attachments` /
    /// [`Shove`] `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of which
    /// author it — deserializes and spawns BYTE-IDENTICAL. When present,
    /// [`into_bundle`](WeaponSpec::into_bundle) carries it into the resolved
    /// [`WeaponSpawnSiblings`] as the `on_death` sibling the wielded-weapon scene seam composes
    /// onto the weapon entity (as an [`OnDeath`](crate::effects::on_death::OnDeath) component), so
    /// [`resolve_on_death`](crate::effects::on_death::resolve_on_death) fans it when the ganger dies.
    #[serde(default)]
    pub on_death:    Option<crate::effects::on_death::OnDeathEffect>,
}

impl WeaponSpec {
    /// Resolve this authored spec into a spawnable [`WeaponBundle`] plus its
    /// [`WeaponSpawnSiblings`] (the optional `dot` / `on_death` sibling components), supplying
    /// the [`WeaponName`] from the registry KEY (the weapon file's filename stem).
    ///
    /// Groups the per-hit damage fields into a [`DamageProfile`] and the
    /// magazine/fire-mode/`stable`/`shove` fields into a [`HandlingProfile`], calls
    /// [`WeaponBundle::new`] (the [`Weapon`](super::Weapon) marker is added there), and returns
    /// the resolved bundle alongside a [`WeaponSpawnSiblings`] carrying the spec's optional
    /// `dot` / `on_death` siblings. The GTW-549 [`attachments`](WeaponSpec::attachments) list is
    /// NOT folded here — attachment effects are applied to the SPAWNED weapon entity via the
    /// [`attach_to_weapon`](crate::equipment::attachments::AttachToWeaponExt::attach_to_weapon) commands extension at
    /// setup (SUPERSEDES the GTW-542 pre-spawn leaf-fold model). A weapon with no attachments,
    /// `dot`, or `on_death` resolves BYTE-IDENTICAL to before the attachment model existed.
    ///
    /// The spawned [`Magazine`] is built FULL (loaded to `size`) from the authored `size` +
    /// `reload_tu` (the GTW-275 "full magazine at spawn" path). Consumes the spec by value (it
    /// owns the [`FireMode`]); a caller holding a borrowed spec clones it first (the registry's
    /// specs are `Clone`).
    #[must_use]
    pub fn into_bundle(self, name: WeaponName) -> (WeaponBundle, WeaponSpawnSiblings) {
        let magazine = Magazine::loaded(self.magazine.size(), self.magazine.reload_tu());
        let bundle = WeaponBundle::new(
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
            )
            .with_trajectory(self.trajectory),
        );
        // GTW-544 / GTW-547: carry the authored `dot` / `on_death` optional siblings the
        // wielded-weapon scene seam composes onto the weapon entity. A `None` (a weapon with no
        // DOT / death effect) leaves the sibling record identity, so the weapon spawns
        // byte-identical.
        let siblings = WeaponSpawnSiblings::new(self.dot, self.on_death);
        (bundle, siblings)
    }
}

/// The resolved spawn-side **optional sibling components** of a weapon spec — the `dot` /
/// `on_death` siblings the wielded-weapon scene seam composes onto the spawned weapon entity
/// (GTW-549; the slimmed successor of the GTW-542 `AttachmentEffects` accumulator, now that
/// attachment effects are applied via the [`attach_to_weapon`](crate::equipment::attachments::AttachToWeaponExt::attach_to_weapon)
/// commands extension rather than pre-spawn leaf rewrites).
///
/// Built by [`into_bundle`](WeaponSpec::into_bundle) from the spec's optional `dot` /
/// `on_death` fields. An identity record (both `None`) composes NO sibling, so a weapon with
/// neither spawns byte-identical.
///
/// NOT `Copy` (the `on_death` field carries an [`OnDeath`](crate::effects::on_death::OnDeath) whose
/// `OnDeathEffect::LeaveField` owns a `FieldKey` [`String`]); it is `Clone`, moved through the
/// spawn seam once.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WeaponSpawnSiblings {
    /// The weapon's [`DotProfile`](super::DotProfile) sibling to add (GTW-544), or `None`
    /// when the weapon authors no `dot:` profile — a penetrating hit then attaches a
    /// [`Dot`](super::Dot) on the struck ganger.
    dot:      Option<DotProfile>,
    /// The weapon's [`OnDeath`](crate::effects::on_death::OnDeath) sibling to add (GTW-547), or `None`
    /// when the weapon authors no `on_death:` effect — the wielding ganger's death then fans
    /// it via [`resolve_on_death`](crate::effects::on_death::resolve_on_death).
    on_death: Option<crate::effects::on_death::OnDeath>,
}

impl WeaponSpawnSiblings {
    /// Build the sibling record from the spec's optional `dot` profile + `on_death` effect
    /// (the `on_death` effect is wrapped into an [`OnDeath`](crate::effects::on_death::OnDeath)).
    #[must_use]
    pub fn new(
        dot: Option<DotProfile>,
        on_death: Option<crate::effects::on_death::OnDeathEffect>,
    ) -> Self {
        Self {
            dot,
            on_death: on_death.map(crate::effects::on_death::OnDeath::new),
        }
    }

    /// The weapon's [`DotProfile`](super::DotProfile) sibling to spawn on the weapon entity
    /// (GTW-544), or `None` when the weapon authors no DOT.
    #[must_use]
    pub const fn dot(&self) -> Option<DotProfile> {
        self.dot
    }

    /// The weapon's [`OnDeath`](crate::effects::on_death::OnDeath) sibling to spawn on the weapon entity
    /// (GTW-547), or `None` when the weapon authors no death effect.
    #[must_use]
    pub const fn on_death(&self) -> Option<&crate::effects::on_death::OnDeath> {
        self.on_death.as_ref()
    }
}

/// The **pending attachment effects** a spawned weapon entity carries until the GTW-549
/// attachment application system applies them — the deferred-spawn bridge that lets the
/// setup path point a weapon at data-driven attachment items and have their effects land on
/// the (deferred-materializing) weapon entity via the
/// [`attach_to_weapon`](crate::equipment::attachments::AttachToWeaponExt::attach_to_weapon) commands extension.
///
/// The wielded-weapon scene is spawned via
/// [`queue_spawn_related_scenes`](bevy::scene::EntityCommandsSceneExt::queue_spawn_related_scenes),
/// whose components materialize deferred (on the `SpawnScene` schedule), so the setup path
/// has no live weapon [`Entity`](bevy::prelude::Entity) to `attach_to_weapon` inline. Instead
/// the resolved item effects ride onto the weapon as THIS component (composed into the scene),
/// and the [`apply_pending_attachments`](crate::equipment::attachments::apply_pending_attachments) system
/// — running AFTER the weapon materializes — reads it and calls `attach_to_weapon(weapon,
/// effect)` for each effect (the mandated post-spawn `EntityCommand`), then removes the marker.
///
/// A named newtype [`Component`] over the resolved effect list (no-bare-types: the pending
/// set is a domain value, not a bare `Vec`). `Default` (an empty list) is the identity — a
/// weapon with no attachments carries it empty and the apply system is a no-op. Private inner
/// with a ctor + [`effects`](PendingAttachments::effects) accessor.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct PendingAttachments(Vec<AttachmentEffect>);

impl PendingAttachments {
    /// Build the pending-attachment marker from the resolved effect list of a weapon's
    /// fitted attachment items.
    #[must_use]
    pub const fn new(effects: Vec<AttachmentEffect>) -> Self {
        Self(effects)
    }

    /// The resolved [`AttachmentEffect`]s to apply to the weapon entity — read by the
    /// application system, which invokes each via the
    /// [`attach_to_weapon`](crate::equipment::attachments::AttachToWeaponExt::attach_to_weapon) extension.
    #[must_use]
    pub fn effects(&self) -> &[AttachmentEffect] {
        &self.0
    }
}
