//! The ganger↔weapon ECS **relationship** pair (ADR-0004,
//! `docs/decisions/0004-equipment-as-entities-relationships.md`): the [`WieldedBy`]
//! back-reference on a weapon entity and the [`Wields`] forward collection on the
//! ganger.
//!
//! GTW-323 slice 2 models a ganger's weapon as its own entity related to the ganger,
//! rather than the GTW-200 weapon-stat components living directly on the ganger
//! entity. One [`Wields`] relation per ganger collects the (single, this slice) wielded
//! weapon entity, which carries the full GTW-200 decomposed weapon-stat component set:
//! the [`Weapon`](super::Weapon) marker, each per-number component, the
//! [`WeaponName`](super::WeaponName), the [`Magazine`](crate::magazine::Magazine)
//! grouping, the [`FireMode`](super::FireMode), and the [`Stable`](super::Stable) tag.
//! So the firing read is `ganger → Wields → the weapon entity → its weapon-stat
//! components`, a keyed traversal mirroring the slice-1 armor
//! [`Wears`](crate::armor::Wears) lookup.
//!
//! `linked_spawn` on [`Wields`] makes the wielded weapon **battle-local**: despawning a
//! ganger cascade-despawns its weapon entity, so equipment lives and dies with the
//! ganger for the battle (ADR-0004 §Consequences) — the same despawn-cascade the armor
//! side uses.
//!
//! These are pure `bevy_ecs` relationship components with no rendering dependency —
//! fully usable in the render-free, headless sim (`MinimalPlugins`); the presenter's
//! migration to read the wielded weapon through [`Wields`] (the weapon panel) is
//! GTW-323 slice 3.

use bevy::{
    ecs::relationship::RelationshipTarget,
    prelude::{Component, Entity},
};

/// The **back-reference** component on a weapon entity — which ganger [`Wields`] this
/// weapon (the weapon→ganger edge of the ADR-0004 weapon relationship).
///
/// A Bevy [`Relationship`](bevy::ecs::relationship::Relationship)
/// (`#[relationship(relationship_target = Wields)]`): inserting it on a weapon entity
/// automatically maintains the matching [`Wields`] collection on the named ganger (the
/// framework's bidirectional back-reference upkeep). It is the component the `bsn!`
/// equipment spawn site relates the weapon to its ganger through (GTW-322
/// `queue_spawn_related_scenes::<Wields>`), mirroring the armor side's
/// [`WornBy`](crate::armor::WornBy).
///
/// The inner [`Entity`] is **Bevy-mandated framework plumbing**, NOT a domain value:
/// the [`Relationship`](bevy::ecs::relationship::Relationship) trait contract
/// (`get(&self) -> Entity` / `from(Entity)`) requires the relationship component to be
/// a newtype over the related [`Entity`]. This is the framework carve-out the
/// no-bare-types rule permits (an entity handle, not a wrapped scalar).
///
/// The inner field is **private** (no-bare-types rule 5): the `#[relationship]` derive's
/// generated trait impl reads it from inside this module, so a private field satisfies
/// the framework without leaking `Self(x)` tuple construction to other crates. Construct
/// it through [`WieldedBy::new`] (or let the framework's related-spawn machinery insert
/// it via [`Relationship::from`](bevy::ecs::relationship::Relationship::from)); read the
/// related ganger through [`Relationship::get`](bevy::ecs::relationship::Relationship::get).
///
/// `Default` ([`Entity::PLACEHOLDER`]) is a **spawn-seed sentinel only** — the
/// `bsn!`-scene spawn path seeds the component slot via `Default` before the
/// related-spawn wiring overwrites it with the real ganger handle (GTW-322). A
/// placeholder entity is never a live edge; a related weapon always carries its real
/// ganger back-reference.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[relationship(relationship_target = Wields)]
pub struct WieldedBy(Entity);

impl WieldedBy {
    /// A back-reference pointing at the ganger that wields this weapon entity.
    ///
    /// The cross-crate constructor for the private inner [`Entity`] (no-bare-types
    /// rule 5): test and spawn sites outside this module relate a weapon to its ganger
    /// through this rather than a `WieldedBy(entity)` tuple literal. `entity` is Bevy
    /// framework plumbing (an entity handle), so a bare-`Entity` signature is the
    /// no-bare-types framework carve-out.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

impl Default for WieldedBy {
    /// The spawn-seed sentinel: [`Entity::PLACEHOLDER`] (not a live edge). `Entity`
    /// has no `Default`, so the sentinel is supplied by hand here (GTW-322), mirroring
    /// [`WornBy`](crate::armor::WornBy).
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// The **forward collection** component on a ganger entity — the weapon entity it
/// [`Wields`](WieldedBy) (the ganger→weapon edge of the ADR-0004 weapon relationship).
///
/// A Bevy [`RelationshipTarget`] (`#[relationship_target(relationship = WieldedBy,
/// linked_spawn)]`): the framework keeps the inner collection in lockstep with the
/// [`WieldedBy`] back-references that point at this ganger — a weapon spawned-and-related
/// via [`WieldedBy`] appears here automatically, in insertion order. `linked_spawn`
/// makes the weapon battle-local: despawning the ganger cascade-despawns its weapon
/// entity (ADR-0004 §Consequences).
///
/// A ganger [`Wields`] a **single** weapon this slice (the collection holds one entity),
/// so the firing read takes the [`first`](Wields::weapon) related entity — the
/// collection type leaves room for the multi-weapon loadout the ADR anticipates without
/// reshaping the relationship.
///
/// The inner `Vec<Entity>` is **Bevy-mandated framework plumbing** — the
/// [`RelationshipTarget`] collection type — NOT a domain value (the no-bare-types
/// framework carve-out). It is read through the [`RelationshipTarget::iter`] trait
/// method (or the [`weapon`](Wields::weapon) accessor), never a bare `.0` index.
/// `Default` (an empty `Vec`) derives cleanly and doubles as the GTW-322 spawn-seed
/// sentinel; the framework populates it as the weapon is related.
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = WieldedBy, linked_spawn)]
pub struct Wields(Vec<Entity>);

impl Wields {
    /// The **single** weapon entity this ganger wields — the first (this slice, only)
    /// related weapon [`Entity`], or `None` if the ganger wields nothing (unarmed /
    /// not yet related).
    ///
    /// The firing read keys `ganger → Wields → the weapon entity` through this. Reads
    /// through the [`RelationshipTarget`] trait so the inner `Vec` stays private
    /// (no-bare-types: the framework collection is reached only through the trait
    /// surface, never a `.0` field access). The collection holds one weapon this slice;
    /// the accessor takes the first so a future multi-weapon loadout (the ADR open
    /// question) can pick the active one without reshaping callers.
    ///
    /// **GTW-505**: with the melee-weapon model live (a ganger wields BOTH a ranged and a
    /// melee weapon), the FIRST related entity is no longer guaranteed to be the ranged
    /// one — the RANGED-firing path must resolve through
    /// [`ranged_weapon`](Wields::ranged_weapon) (which excludes
    /// [`MeleeWeapon`](super::MeleeWeapon)) instead. `weapon` is retained for callers that
    /// genuinely want the first-related entity regardless of kind (e.g. a test asserting
    /// any wielded entity exists).
    #[must_use]
    pub fn weapon(&self) -> Option<Entity> {
        self.iter().next()
    }

    /// The **ranged** weapon entity this ganger wields — the first related entity that is
    /// NOT a [`MeleeWeapon`](super::MeleeWeapon), determined by the caller-supplied
    /// `is_melee` predicate; or `None` when the ganger wields no ranged weapon (GTW-505 C5).
    ///
    /// This is the ranged-firing resolution after the melee model went live: a ganger
    /// wields BOTH a ranged weapon entity (carrying [`Weapon`](super::Weapon)) AND a melee
    /// weapon entity (carrying [`MeleeWeapon`](super::MeleeWeapon)), both in this collection
    /// — so [`weapon`](Wields::weapon) (the FIRST entity) is no longer safe for the ranged
    /// path. The caller backs `is_melee` with a `Query<(), With<MeleeWeapon>>` (`|e|
    /// melee.get(e).is_ok()`), and this returns the first NON-melee related entity — the
    /// ranged weapon `fire()` / `can_fire` / the reaction + AI paths read. So relating a
    /// melee weapon NEVER regresses ranged firing (the zero-ranged-regression mechanism).
    ///
    /// `is_melee` is a `Fn(Entity) -> bool` (NOT a domain newtype) because it is a
    /// caller-injected query CLOSURE — Bevy framework plumbing the no-bare-types rule's
    /// trait-impl / system-param carve-out covers, not a wrapped domain scalar.
    #[must_use]
    pub fn ranged_weapon(&self, is_melee: impl Fn(Entity) -> bool) -> Option<Entity> {
        self.iter().find(|&entity| !is_melee(entity))
    }

    /// The **melee** weapon entity this ganger wields — the first related entity that IS a
    /// [`MeleeWeapon`](super::MeleeWeapon), determined by the caller-supplied `is_melee`
    /// predicate; or `None` when the ganger wields no melee weapon (GTW-505). The
    /// counterpart to [`ranged_weapon`](Wields::ranged_weapon): the GTW-506/507 melee
    /// resolution reads its weapon through this. Since every spawned ganger relates a
    /// melee weapon (an authored one OR the [`fists`](super::FISTS_KEY) default — the
    /// GTW-37 D3 ruling), this resolves to `Some` for every fielded ganger.
    ///
    /// `is_melee` is the same caller-injected query CLOSURE
    /// [`ranged_weapon`](Wields::ranged_weapon) takes (the framework carve-out).
    #[must_use]
    pub fn melee_weapon(&self, is_melee: impl Fn(Entity) -> bool) -> Option<Entity> {
        self.iter().find(|&entity| is_melee(entity))
    }
}
