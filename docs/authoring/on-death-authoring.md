# On-Death Effect Authoring Guide

How to author what a dying thing DOES — the `on_death:` list a weapon or a
terrain piece carries. A volatile satchel charge detonating on its wielder's
corpse, a fuel drum leaving burning ground when smashed — both are one authored
field, no code.

---

## Part 1 — Authoring `on_death:` (content)

### 1a. The two authoring homes

`on_death:` is a LIST of effects (`#[serde(default)]` → an empty list) on two
specs. Every effect in the list fires, in the order it is written in the file:

| Home | File family | Fans when |
|------|-------------|-----------|
| `WeaponSpec.on_death` | `assets/content/weapons/ranged/*.weapon.ron` | The WIELDING ganger dies |
| `TerrainDef.on_death` | `assets/content/terrain/<theme>/*.terrain_def.ron` | The piece is DESTROYED |

There is no separate on-death file family — the effect rides the spec that
owns the dying source.

### 1b. The two effect variants — by example

`OnDeathEffect` (`crates/gdtf_battle_sim/src/effects/on_death/effect.rs`) is a
closed enum with two shipped variants (struct variants — single-paren
named-field RON form). Magnitudes below are shape examples, not tuned values.

**`Explode`** — fan a deterministic blast at the death cell. From the shipped
`assets/content/weapons/ranged/volatile_charge.weapon.ron`:

```ron
on_death: [
    Explode(
        hit_type: Blast(radius: 1), // AoE template: Blast / Cone / Line / Single
        damage:      8,             // flat HP drained per affected ganger (no RNG, bypasses armor)
        damage_type: Blast,         // wheel-node flavour (presentation only)
    ),
],
```

**`LeaveField`** — spawn a persistent field at the death cell. From the
shipped `assets/content/terrain/sump_waste/waste_drum.terrain_def.ron`:

```ron
on_death: [
    LeaveField(
        field: "toxic_waste_pool",  // FieldKey — a `assets/content/fields/<key>.field.ron` stem
    ),
],
```

A list may hold both, and both fire:

```ron
on_death: [
    Explode(hit_type: Blast(radius: 1), damage: 8, damage_type: Blast),
    LeaveField(field: "toxic_waste_pool"),
],
```

The `field:` key resolves against the `FieldDefRegistry` at fan time
([field-authoring.md](field-authoring.md)); a dangling key fans nothing at all,
and the fan itself logs nothing. It surfaces on the `Load` reference-integrity
report instead. Both hosts check the same key at load, the game and the content
editor, so a dangling one is a finding rather than a silent no-op.

### 1c. The runtime contract (what an author can rely on)

- **Every terminal gate emits.** `OnDeathOccurred`
  (`crates/gdtf_battle_sim/src/effects/on_death/signal.rs`) fires from every
  path that kills — ranged fire (primary + splash), melee kills, falls, the
  per-round bleed / DOT / field clocks, both cover-destroy sites, and the
  slab-destroy site — so an authored effect fans no matter HOW the source died.
- **Deterministic, RNG-free.** The `Explode` drain is a flat, armor-bypassing
  HP drain over the sorted AoE set — byte-stable, so it never perturbs the
  seeded autobattle stream.
- **Cascades resolve to a fixpoint.** A blast that kills another on-death
  carrier pushes a fresh `OnDeathOccurred` onto the resolver's same-frame
  work-queue (`resolve_on_death`,
  `crates/gdtf_battle_sim/src/effects/on_death/resolve.rs`) — chained
  explosions all land.
- **Every effect in the list fires, in the authored order.** The resolver fans
  one death's whole list front to back before it takes the next death off its
  queue.
- **Two pieces on one cell each fan.** A terrain death is deduped by WHICH
  piece died (cover and slab are separate keys); an entity death is deduped by
  its cell. So a cover and a slab destroyed on the same cell in the same frame
  each fan every effect their own def authors.
- **A mounted gun's list is the one that fans.** An emplacement's mounted
  weapon carries the `on_death:` its own spec authors, and the death read picks
  the mounted gun over the carried one, so a gunner killed at the mount fans the
  mount's list.
- A cone authored on a corpse degenerates to the full disc (a corpse has no
  meaningful fire direction).

---

## Part 2 — Adding a new on-death effect (engineers)

The palette (`crates/gdtf_battle_sim/src/effects/on_death/`) isolates each
variant's behaviour in its own file (`explode.rs` / `leave_field.rs`) behind
the `ApplyOnDeathEffect` trait; the enum's own impl is the one mechanical
delegation match, and the resolver invokes the trait generically. Adding an
effect costs:

1. ONE per-effect file (payload newtype + isolated `ApplyX` type +
   `impl ApplyOnDeathEffect` + a `#[cfg(test)]` unit test),
2. ONE `OnDeathEffect` variant + ONE delegation arm (exhaustive match — the
   compiler forces it),
3. ONE `mod` line in the family's `mod.rs`.

`DeathFanOut`, the borrowed fan-out each effect writes through, is defined in
`crates/gdtf_battle_sim/src/effects/on_death/apply_effect.rs` alongside the
`ApplyOnDeathEffect` trait. The message every kill path emits is
`OnDeathOccurred` in that folder's `signal.rs`, and 1c names the paths that
write it.

---

## Part 3 — Verify

- **Suite:** `cargo dtest`. The mechanics suite lives in-crate under
  `crates/gdtf_battle_sim/src/effects/on_death/test/` (resolver, cascade, drain
  discipline, per-piece keying, the per-round clocks) alongside `tests.rs` (the
  shipped serde forms and the delegation), plus the per-effect unit tests in
  `explode.rs` / `leave_field.rs`. The end-to-end fans — a smashed barrel, a
  destroyed slab, a def authoring two effects that both land — are driven
  through a real battle in `crates/gdtf_battle_sim/tests/ondeath_effects/`.
- **In game:** `cargo drun` — the shipped skirmish fields "Alex Mercer" with
  the `volatile_charge` satchel weapon: when Alex dies, the radius-1 blast
  detonates at the body (the presenter marks it with a bold "BOOM" floating
  text at the death cell). Smashing a `waste_drum` piece (sump_waste theme)
  leaves a toxic field on the cell.
