//! The ganger↔armor-piece ECS **relationship** pair (ADR-0004,
//! `docs/decisions/0004-equipment-as-entities-relationships.md`): the [`WornBy`]
//! back-reference on each armor-piece entity and the [`Wears`] forward collection
//! on the ganger.
//!
//! GTW-323 slice 1 models a ganger's armor as its own entities related to the
//! ganger, rather than the single `WornArmor([ArmorPiece; 6])` blob component the
//! ADR supersedes. One [`Wears`] relation per ganger collects the six worn-piece
//! entities, each tagged with its [`BodyPart`](super::BodyPart) (ADR-0004 §Decision:
//! "one entity per worn `ArmorPiece`, each tagged with its `BodyPart`") — so the
//! `struck_piece` armor lookup is `ganger → Wears → the BodyPart-tagged piece
//! entity → its ArmorPiece components`, a keyed traversal, NOT a per-slot relation
//! variant (the ADR open question, resolved here to one relation + a `BodyPart`
//! tag; confirmed cleaner at the lookup site).
//!
//! `linked_spawn` on [`Wears`] makes the worn pieces **battle-local**: despawning a
//! ganger cascade-despawns its piece entities, so equipment lives and dies with the
//! ganger for the battle (ADR-0004 §Consequences). The weapon side
//! (`WieldedBy`/`Wields`) is GTW-323 slice 2; this slice is armor only.
//!
//! These are pure `bevy_ecs` relationship components with no rendering dependency —
//! fully usable in the render-free, headless sim (`MinimalPlugins`); the
//! presenter's migration to read worn pieces through [`Wears`] is GTW-323 slice 3.

use bevy::{
    ecs::{query::QueryData, relationship::RelationshipTarget},
    prelude::{Component, Entity},
};

use super::stats::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
};

/// The **back-reference** component on one armor-piece entity — which ganger
/// [`Wears`] this piece (the piece→ganger edge of the ADR-0004 armor relationship).
///
/// A Bevy [`Relationship`](bevy::ecs::relationship::Relationship)
/// (`#[relationship(relationship_target = Wears)]`): inserting
/// it on a piece entity automatically maintains the matching [`Wears`] collection on
/// the named ganger (the framework's bidirectional back-reference upkeep). It is the
/// component the `bsn!` equipment spawn site relates each piece to its ganger through
/// (GTW-322 `queue_spawn_related_scenes::<Wears>`).
///
/// The inner [`Entity`] is **Bevy-mandated framework plumbing**, NOT a domain value:
/// the [`Relationship`](bevy::ecs::relationship::Relationship) trait contract
/// (`get(&self) -> Entity` / `from(Entity)`)
/// requires the relationship component to be a newtype over the related [`Entity`].
/// This is the framework carve-out the no-bare-types rule permits (an entity handle,
/// not a wrapped scalar).
///
/// The inner field is **private** (no-bare-types rule 5): the `#[relationship]` derive's
/// generated trait impl reads it from inside this module, so a private field satisfies
/// the framework without leaking `Self(x)` tuple construction to other crates. Construct
/// it through [`WornBy::new`] (or let the framework's related-spawn machinery insert it
/// via [`Relationship::from`](bevy::ecs::relationship::Relationship::from)); read the
/// related ganger through [`Relationship::get`](bevy::ecs::relationship::Relationship::get).
///
/// `Default` ([`Entity::PLACEHOLDER`]) is a **spawn-seed sentinel only** — the
/// `bsn!`-scene spawn path seeds the component slot via `Default` before the
/// related-spawn wiring overwrites it with the real ganger handle (GTW-322). A
/// placeholder entity is never a live edge; a related piece always carries its real
/// ganger back-reference.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[relationship(relationship_target = Wears)]
pub struct WornBy(Entity);

impl WornBy {
    /// A back-reference pointing at the ganger that wears this armor-piece entity.
    ///
    /// The cross-crate constructor for the private inner [`Entity`] (no-bare-types
    /// rule 5): test and spawn sites outside this module relate a piece to its ganger
    /// through this rather than a `WornBy(entity)` tuple literal. `entity` is Bevy
    /// framework plumbing (an entity handle), so a bare-`Entity` signature is the
    /// no-bare-types framework carve-out.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

impl Default for WornBy {
    /// The spawn-seed sentinel: [`Entity::PLACEHOLDER`] (not a live edge). `Entity`
    /// has no `Default`, so the sentinel is supplied by hand here (GTW-322).
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// The **forward collection** component on a ganger entity — the worn armor-piece
/// entities it [`Wears`](WornBy) (the ganger→pieces edge of the ADR-0004 armor
/// relationship).
///
/// A Bevy [`RelationshipTarget`] (`#[relationship_target(relationship = WornBy,
/// linked_spawn)]`): the framework keeps the inner collection in lockstep with the
/// [`WornBy`] back-references that point at this ganger — a piece spawned-and-related
/// via [`WornBy`] appears here automatically, in insertion order. `linked_spawn`
/// makes the pieces battle-local: despawning the ganger cascade-despawns every worn
/// piece (ADR-0004 §Consequences).
///
/// The inner `Vec<Entity>` is **Bevy-mandated framework plumbing** — the
/// [`RelationshipTarget`] collection type — NOT a domain value (the no-bare-types
/// framework carve-out). It is read through the [`RelationshipTarget::iter`] trait
/// method (or [`collection`](RelationshipTarget::collection)), never a bare `.0`
/// index. `Default` (an empty `Vec`) derives cleanly and doubles as the GTW-322
/// spawn-seed sentinel; the framework populates it as pieces are related.
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = WornBy, linked_spawn)]
pub struct Wears(Vec<Entity>);

impl Wears {
    /// The worn-piece entities this ganger [`Wears`](WornBy), in insertion order — a
    /// borrowing iterator over the related-piece [`Entity`] handles.
    ///
    /// The keyed `struck_piece` lookup (`ganger → Wears → the BodyPart-tagged piece`)
    /// iterates these, matching each piece's [`BodyPart`](super::BodyPart) tag. Reads
    /// through the [`RelationshipTarget`] trait so the inner `Vec` stays private
    /// (no-bare-types: the framework collection is reached only through the trait
    /// surface, never a `.0` field access).
    pub fn pieces(&self) -> impl Iterator<Item = Entity> + '_ {
        self.iter()
    }
}

/// The **mutable per-piece armor view** the hit pipeline reads a struck worn piece
/// through — the [`QueryData`] for one armor-piece entity's [`BodyPart`] tag, its
/// four read-only stats, and its **one mutable** [`ArmorIntegrity`] wear field
/// (GTW-323 slice 1, ADR-0004).
///
/// `struck_piece` reads this view (the matchup + the per-hit damage formula run
/// against the four stats) and `wear_armor` mutates the [`integrity`](PieceArmorMut::integrity)
/// component in place — the piece-entity replacement for the old `WornArmor`
/// array-slot read+wear. Only [`ArmorIntegrity`] is `&mut` (the single wear field;
/// hardness/protection/floor/type do not degrade, matching the worn-copy immutability
/// of `weapons-and-armor.md`). Queried `With<`[`WornBy`]`>` so only worn pieces match.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct PieceArmorMut {
    /// The body location this piece protects — the tag the `struck_piece` lookup keys on.
    pub part:       &'static BodyPart,
    /// The piece's minimum-damage floor (read-only).
    pub floor:      &'static ArmorFloor,
    /// The piece's damage-soak protection (read-only).
    pub protection: &'static ArmorProtection,
    /// The piece's durability — the **one mutable** wear field (degrades per hit).
    pub integrity:  &'static mut ArmorIntegrity,
    /// The piece's penetration-ignoring hardness (read-only; does not degrade).
    pub hardness:   &'static ArmorHardness,
    /// The piece's matchup-wheel node (read-only).
    pub armor_type: &'static ArmorType,
}
