---
name: "ADR 0004: Equipment as entities related to the ganger"
description: Model weapons and armor as their own entities related to the ganger via Bevy ECS relationships, not as components stored on the ganger.
---

# 0004. Equipment as entities related to the ganger

## Status

`Proposed` — 2026-06-20, driven by [GTW-323](https://linear.app/robert-gardner/issue/GTW-323).

Awaiting review/acceptance before the remodel is built. The implementation is
intentionally **not** started: this ADR is the design pass GTW-323 calls for.

## Context

GDTF's core loop is *fight → consequences on survivors → carry the scarred
roster forward → fight again, changed* ([../index.md](../index.md)). Persistent
gear across battles — loot it, transfer it between gangers, drop it, pick it up,
mod it ([GTW-41](https://linear.app/robert-gardner/issue/GTW-41)), carry it onto
the next map — is a first-class part of that loop, not a battle-only detail.

Today equipment is **stored on the ganger entity**, and inconsistently:

- **Armor** is a single `WornArmor([ArmorPiece; 6])` component on the ganger —
  a fixed-size array of the six body-part pieces (`armor/worn.rs`).
- **Weapon** is *decomposed into a bundle of components on the ganger* — a
  `Weapon` marker plus one stat component per number, plus `WeaponName`,
  `Magazine`, `FireMode`, etc. (the
  [GTW-198](https://linear.app/robert-gardner/issue/GTW-198) decomposition).

So armor is one packed component and the weapon is a scatter of components, both
living **on the ganger**. Neither models a *thing the ganger has* that could be
removed, swapped, transferred, or extended (mods) — they are properties of the
ganger, copied as blobs.

The data-driven loaders for both already landed:
[GTW-257](https://linear.app/robert-gardner/issue/GTW-257) (`WeaponRegistry` +
`WeaponSpec` from `assets/weapons/*.weapon.ron`) and
[GTW-269](https://linear.app/robert-gardner/issue/GTW-269) (`ArmorRegistry` +
`ArmorSpec` from `assets/armor/*.armor.ron`). `setup_battle` currently resolves a
ganger's weapon/armor *keys* against those registries and **seeds the resolved
values onto the ganger** (a `WeaponBundle` insert; `WornArmor::seed_from`).

Bevy 0.18 has first-class **ECS relationships** (a `Relationship` component +
its `RelationshipTarget` collection, with automatic back-reference maintenance
and despawn-cascade). That makes "a ganger *has* these equipment entities" an
ECS-native concept rather than something we hand-roll with stored `Entity`
handles.

`docs/combat/weapons-and-armor.md` defines the *stat vocabulary* (weapon:
damage/punch/shred/damage_type; armor piece: floor/protection/integrity/hardness
/armor_type) but is silent on *where* equipment lives in the ECS — that is an
architecture choice, which is what this ADR settles.

## Decision

**We will model equipment as its own entities, related to the ganger via Bevy
ECS relationships, instead of as components stored on the ganger.**

- `ganger --Wears--> {head, torso, left_arm, right_arm, left_leg, right_leg}`
  armor-piece entities — one entity per worn `ArmorPiece`, each tagged with its
  `BodyPart`. The piece's stat components (`ArmorFloor`/`ArmorProtection`/
  `ArmorIntegrity`/`ArmorHardness`/`ArmorType`) live **on the piece entity**.
- `ganger --Wields--> weapon` entity — the weapon's stat components (the
  GTW-198 decomposition) move **off the ganger onto the weapon entity**, leaving
  room for a future mod sub-hierarchy (`weapon --HasMod--> mod` entities,
  GTW-41) and for multiple wielded/stowed weapons + throwables as additional
  related entities.

Concretely: define custom relationship component pairs (e.g. `WornBy`/`Wears`
for armor pieces, `WieldedBy`/`Wields` for the weapon — exact derive syntax to
be confirmed against the pinned Bevy 0.18 API during the build). `setup_battle`
**spawns** an armor-piece entity per `ArmorSpec` piece and a weapon entity from
the `WeaponSpec`, then **relates** them to the ganger — instead of inserting a
`WornArmor`/`WeaponBundle` onto the ganger. The
[GTW-257](https://linear.app/robert-gardner/issue/GTW-257)/[GTW-269](https://linear.app/robert-gardner/issue/GTW-269)
registries → spec lookups are **reused unchanged**; only the terminal "seed onto
the ganger" step becomes "spawn equipment entity + relate".

This supersedes the [GTW-198](https://linear.app/robert-gardner/issue/GTW-198)
direction for *where* the decomposed weapon components live (they move from the
ganger onto the weapon entity); the decomposition itself (one component per
number, queryable) stays.

## Consequences

**Enabled (the point):**

- Loot / transfer / drop / pick-up / persistent gear across battles become
  **re-targeting a relationship** (or re-parenting an entity), not copying
  component blobs. Aligns the ECS shape with the campaign/roster layer
  ([GTW-67](https://linear.app/robert-gardner/issue/GTW-67) equipment & loadout,
  [GTW-100](https://linear.app/robert-gardner/issue/GTW-100) gear catalog).
- Weapon modding/attachments ([GTW-41](https://linear.app/robert-gardner/issue/GTW-41))
  becomes a sub-hierarchy hanging off the weapon entity; multiple weapons +
  throwables become additional related entities — neither expressible cleanly
  with components-on-the-ganger.
- Removes the armor-vs-weapon asymmetry: both are entities the ganger relates to.

**Cost / new constraints:**

- **Refactor breadth is the real cost** (not determinism — see below). Touch
  points: `resolve_hit`/`struck_piece` (the struck part's armor lookup becomes
  ganger → `Wears` → the `BodyPart`-tagged piece entity → its `ArmorPiece`
  components); `apply_hit`/`armor_wear::wear_armor` (mutates the *piece entity's*
  `ArmorIntegrity` component, not an array slot); `fire()` (reads the *wielded
  weapon entity's* components via `Wields`); `setup_battle` (spawn + relate);
  the weapon-panel / status-panel presenters (read the ganger's wielded weapon /
  worn pieces via the relationships); and every ganger-building test harness.
- The lookup is no longer an O(1) array index — it's a relationship traversal +
  a keyed `Query::get` on the piece entity. Cheap, but a new access pattern.

**Determinism — NOT a risk (this corrects GTW-323's original framing):** the
seeded byte-equal-volley property holds. The sim reads equipment by **key**
(part → piece, ganger → weapon), never by order-dependent iteration over a
component array, so the looked-up value is identical regardless of entity
storage. RNG draws are per-round sequential (the march draws none), and the sim
already avoids `Entity`-id-order dependence (e.g. `auto_select` sorts by the
`(z, y, x)` cell key). Bevy's `RelationshipTarget` collection is
insertion-ordered. The remodel changes *where a value is stored*, not *what
value a keyed lookup returns* or *the RNG draw order*. **Keep one seeded-replay
regression test as cheap insurance**, but determinism is not the gating concern.

**Mitigations already in place:** [GTW-269](https://linear.app/robert-gardner/issue/GTW-269)
(loaders/registries/specs) and [GTW-324](https://linear.app/robert-gardner/issue/GTW-324)
(central `test_support` builders — `GangerSpawnBuilder`/`SituationBuilder`/
`BattleAppBuilder`) both landed; the latter means the harness ripple from this
remodel is absorbed by the builders rather than ~30 inline test sites.

**Suggested slices (refine when accepted):**

1. Relationship types (`Wears`/`Wears`-target, `Wields`/`Wields`-target) +
   armor-as-entities: `setup_battle` spawns+relates armor pieces; `resolve_hit`/
   `struck_piece` + `apply_hit`/`wear_armor` read/mutate the piece entity; a
   seeded-replay determinism regression test.
2. Weapon-as-entity: the GTW-198 weapon components move onto the weapon entity;
   `fire()` reads them via `Wields`; `setup_battle` spawns+relates the weapon.
3. Presenter reads: weapon-panel / status-panel resolve the ganger's wielded
   weapon / worn pieces through the relationships.

`docs/combat/weapons-and-armor.md` should gain a short "where equipment lives"
note once this is Accepted (link, don't duplicate).

## Open questions (resolve during the build)

- Exact Bevy 0.18 relationship derive syntax + whether one `Wears` relation with
  a per-piece `BodyPart` tag, or per-slot relations, reads cleanest at the
  `struck_piece` lookup. Confirm via the `bevy-expert` at slice 1.
- Whether `WornArmor`/the `WeaponBundle` are fully removed or kept as a transient
  authoring shape — decide once the entity model is in.

## Alternatives considered

- **Keep equipment as components on the ganger (status quo).** Rejected: makes
  loot/transfer/drop/modding/multi-weapon awkward (blob-copying, no sub-tree),
  and keeps the armor-vs-weapon asymmetry. The campaign loop is the whole point
  of the project; the ECS shape should serve it, not just the in-battle math.
- **A single `Equipment` component holding `Vec<…>` / `Entity` handles on the
  ganger.** Rejected: re-implements relationships by hand (manual back-refs, no
  despawn-cascade, no bidirectional query) and still centralizes gear on the
  ganger; the modding sub-tree stays awkward.
- **Entities for equipment but referenced by raw stored `Entity` handles (no
  relationships).** Rejected: loses Bevy's native `Relationship`/
  `RelationshipTarget` machinery (automatic back-reference, cascade, both-ways
  query) for no benefit — relationships are exactly the tool for "ganger has
  these entities".
- **Defer the remodel; build new equipment work on the components-on-ganger
  model.** Rejected for new equipment work, but note the data-driven loaders
  (GTW-257/269) were deliberately built model-agnostic and land *as-is* — only
  the terminal seeding step changes here, so nothing was wasted by sequencing
  the loaders first.
