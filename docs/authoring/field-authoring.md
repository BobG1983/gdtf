# Field Authoring Guide

How to create, extend, and maintain area-damage FIELDS — persistent per-tile
damage zones (a toxic waste pool, an electrified floor, burning ground) that
drain the HP of whoever stands on them, once per round, until they expire
(consequence palette isolated in; runtime merged into the
effects family in).

---

## Part 1 — Creating a new field type (content authoring)

The content editor has a FIELD tab that writes the same
`assets/content/fields/<key>.field.ron` this part describes, so hand-editing the RON is no
longer the only route. Click FIELD in the tab bar to open it; the ten digit hotkeys are
taken by the older tabs, so the Field tab has none. The RON reference below stays the source
of truth for the schema.

### 1a. Where the `.ron` file goes

Field CATALOG entries live under `assets/content/fields/` — one file per field
type, flat. The file is named `<key>.field.ron`; the file stem is the
`FieldKey` every placement references (`toxic_waste_pool.field.ron` → key
`"toxic_waste_pool"`).

### 1b. The `.field.ron` schema — by example

From the shipped `assets/content/fields/toxic_waste_pool.field.ron`
(magnitudes are shape examples, never pinned values):

```ron
(
    damage:             3,                 // FieldDamage — flat HP drained per turn (bypasses armor)
    damage_type:        Chem,              // wheel-node flavour (presentation only — the tick bypasses the matchup)
    immune_armor_types: [Flak, Hazard],    // whole-source immunity: ANY worn piece of these skips ALL damage
    duration:           Permanent,         // never expires; or Turns(3) = exactly 3 draining rounds (Turns(0) is rejected at load)
)
```

**Field reference** (`FieldDef`,
`crates/gdtf_battle_sim/src/effects/fields/field.rs`):

| Field | Rust type | RON form | Notes |
|-------|-----------|----------|-------|
| `damage` | `FieldDamage` | bare integer | Flat per-turn HP drain — no armor matchup, no injury roll, no RNG |
| `damage_type` | `DamageType` | enum variant | Wheel-node flavour, presentation only (the drain bypasses soak) |
| `immune_armor_types` | `ImmuneArmorTypes` | list of `ArmorType` variants | WHOLE-SOURCE immunity: a ganger wearing ANY piece of a listed type takes zero. A ganger wearing no armor at all is never exempt. It is drained like any other unprotected occupant |
| `duration` | `FieldDuration` | `Permanent` \| `Turns(n)` | `Turns(n)` = exactly `n` draining rounds, then the field is removed (the same round boundary that spends the last turn removes it). `n` must be ≥ 1 — `Turns(0)` is unrepresentable and FAILS the file's load (per-file salvage, `MalformedFile` finding); there is no clamp |

### 1c. Placing a field (three producers)

A catalog entry does nothing until something PLACES it at a `(cell, level)`:

1. **A situation** — the authored `fields:` list in
   `assets/content/situations/skirmish.ron` places initial-terrain hazards
   (each entry names a `field:` key + a position). See
   [battlefield-authoring.md](battlefield-authoring.md).
2. **An on-death effect** — `on_death: [LeaveField(field: "<key>")]` on a
   weapon or terrain def spawns the field at the death cell
   ([on-death-authoring.md](on-death-authoring.md)).
3. **Code** — `FieldRegistry::spawn`
   (`crates/gdtf_battle_sim/src/effects/fields/registry.rs`), the API both of
   the above resolve through.

### 1d. The tick contract (what an author can rely on)

`tick_fields` (`crates/gdtf_battle_sim/src/effects/fields/tick.rs`) runs once
per full round (the enemy-phase-start cadence shared with the bleed clock):
each fielded cell drains its occupant (unless an armor exemption applies), then
every placement's lifetime counts down and expired placements are removed. A
`Turns(n)` field therefore drains on exactly `n` round boundaries — the
boundary that spends its last turn still drains, then removes the placement
(never `n + 1`; pinned by the `test_lifetime.rs` suite). The
presenter draws a field overlay on fielded cells
(`crates/gdtf_battle_presenter/src/overlays/field/`) and a hazard-orange FCT
pop on each tick ([fct-authoring.md](fct-authoring.md)); the affliction logs
ONCE at start in the combat log
([combat-log-authoring.md](combat-log-authoring.md)).

---

## Part 2 — How to extend the field model (engineers)

The family lives in one home, palette + mechanics
(`crates/gdtf_battle_sim/src/effects/fields/`):

- **Mechanics** — `field.rs` (the authored `FieldDef`), `registry.rs` (the
  `FieldDefRegistry` catalog + the live per-cell `FieldRegistry`), `tick.rs`
  (the round clock + the `FieldTicked` message).
- **Consequence palette** — one self-contained file per consequence
  (`drain.rs` / `immunity.rs` / `duration.rs`), each implementing the
  `ApplyFieldEffect` trait; the closed `FieldEffect` vocabulary (`effect.rs`)
  delegates mechanically. The authored surface stays the flat `FieldDef`
  struct — `FieldEffect::consequences_of` projects it into the vocabulary, so
  a schema change and a behaviour change stay separate edits.

Adding a new consequence = ONE per-consequence file + ONE `FieldEffect`
variant + ONE delegation arm + ONE `mod` line. The full recipe lives in the
module rustdoc of `crates/gdtf_battle_sim/src/effects/fields/mod.rs` — that
rustdoc is the source of truth; this section is the pointer. A new AUTHORED
field on `FieldDef` additionally follows the standard spec-extension steps
(newtype, `#[serde(default)]` or update every shipped `.field.ron`, projection
into the vocabulary).

---

## Part 3 — Loading and hot-reload

Fields are one of the generic folder-loaded content families
([content-families.md](content-families.md)): the `FieldsFamily` marker
(`crates/gdtf_content_families/src/fields.rs`) declares folder
`content/fields` + extension `field.ron`; one
`register_content_family::<FieldsFamily>()` line in
`crates/gdtf_app/src/states/load/plugin.rs` yields the loader, the
`FieldDefRegistry`, per-file salvage, and the live redrive. Run `cargo drun`,
edit a `.field.ron`, and the registry rebuilds live (an `info!` line names the
reload); the next placement resolves the edited def.

A dangling `field:` key (from a situation or an on-death effect) surfaces on
the end-of-`Load` reference-integrity report — see
[reference-integrity.md](reference-integrity.md).

---

## Part 4 — Verify

- **Suite:** `cargo dtest`. Load coverage:
  `crates/gdtf_app/tests/load_fields.rs` (registry presence + shipped stem,
  value-agnostic). Tick/immunity/duration mechanics: the in-crate tests in
  `crates/gdtf_battle_sim/src/effects/fields/` (per-consequence unit tests +
  the family suite in `test.rs` / `tests.rs`).
- **In game:** `cargo drun` — the shipped skirmish authors one
  `toxic_waste_pool` between the deployments; walk a ganger onto it and end
  the round: the hazard-orange FCT tick pop and the one-time combat-log
  affliction line appear, and the field overlay marks the cell.
