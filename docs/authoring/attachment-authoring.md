# Attachment Authoring Guide

How to create, extend, and maintain weapon attachments — data-driven RON ITEMS
with a typed effect list ( superseding the retired `AttachTag`
enum + global-tuning model; palette re-homed in; slot gating added in
). This guide is the primary reference for content authors adding
attachment items and engineers adding new attachment EFFECTS.

The three design facts that shape everything here:

1. **An attachment is a data file, not code.** A weapon references it BY KEY.
2. **Every magnitude lives ON the item** (a typed payload per effect), never in
   global tuning.
3. **A sight boosts AIM, not stability** — the two are distinct combat levers
   (`docs/combat/resolution.md`).

---

## Part 1 — Creating a new attachment item (content authoring)

### 1a. Where the `.ron` file goes

Attachment items live under `assets/content/attachments/` — one file per item,
flat. The file is named `<key>.attachment.ron`, where `<key>` is the item's
registry key.

**Key convention:** the file stem minus the `.attachment` infix is the item's
`AttachmentName` key — the string a weapon's `attachments:` list references.
For example, `red_dot_sight.attachment.ron` → key `"red_dot_sight"`.

### 1b. The `.attachment.ron` schema — by example

The shipped `assets/content/attachments/extended_mag.attachment.ron` (a
multi-effect item):

```ron
// Extended magazine — adds capacity AND speeds the reload.
(
    display_name: "Extended Magazine",
    slot: Magazine, // the magazine well — fits only a magazine-fed gun declaring a Magazine slot
    effects: [
        ExtraAmmo(8),      // +8 magazine capacity — tunable
        ReloadTime(0.8),   // ×0.8 the weapon's reload_tu (< 1 = faster) — tunable
    ],
)
```

**Field reference** (`AttachmentSpec`,
`crates/gdtf_battle_sim/src/equipment/attachments/spec.rs`):

| Field | Rust type | RON form | Notes |
|-------|-----------|----------|-------|
| `display_name` | `WeaponName` | quoted string | Human-facing picker/HUD label — NOT the key (the key is the file stem) |
| `slot` | `AttachmentSlot` | bare variant | The SINGLE mount point the item occupies (see 1c). REQUIRED — no default |
| `effects` | `Vec<AttachmentEffect>` | list of variants | The typed effect list, each with its per-item magnitude (see 1d). `#[serde(default)]` — omitted = a cosmetic no-op item |

### 1c. `slot:` — the closed slot vocabulary

`AttachmentSlot` (`crates/gdtf_battle_sim/src/equipment/attachments/slot.rs`)
is closed on purpose — a typo'd slot fails to parse:

| Variant | Mount | Typical host |
|---------|-------|--------------|
| `Muzzle` | Muzzle thread | Ranged — suppressors, chokes, bore devices |
| `Sight` | Sight dovetail | Ranged — optics (Aim) |
| `Rail` | Utility rail | Ranged — braces, foregrips, jury-rigged action mods (commonly multi-capacity) |
| `Magazine` | Magazine well | Ranged — box magazines, drums, ammo feeds on a magazine-fed gun |
| `Counterweight` | Counterweight socket | Melee — balance weights |
| `Pommel` | Grip-end pommel | Melee — hilt fittings |

There is deliberately NO ranged/melee class tag on an item — class gating
EMERGES from which slots a weapon declares (a `Muzzle` item finds no slot on a
chainblade). The weapon side authors `slots: [(Muzzle, 1), (Sight, 1),
(Rail, 3)]` — see [weapon-authoring.md](weapon-authoring.md) §1j.

**Magazine slot:** the `Magazine` well is live. Magazine / ammo-feed
items — `extended_mag`, `gore_sump_drum`, `hexgrind_rounds` — occupy it, and
only ranged guns that actually feed from a detachable magazine declare a
`Magazine` slot (energy- and fluid-fed weapons — power cells, plasma flasks,
chem tanks — and melee weapons do not, so their magazine items find no slot and
are cleanly rejected). The `Magazine` SLOT (an attachment mount point) is
distinct from 's `accepts:` AmmoType (what a weapon loads).

### 1d. `effects:` — the closed effect vocabulary

`AttachmentEffect` (`crates/gdtf_battle_sim/src/effects/attachments/effect.rs`)
— every variant, its RON form, and what it moves. Magnitudes shown are shape
examples, never "correct" values:

| Variant (RON form) | Payload | What it does |
|--------------------|---------|--------------|
| `Aim(0.25)` | `AimDelta` | Raises the weapon's `Accuracy` (in-cone concentration) — the sight lever |
| `Stability(20.0)` | `WeaponBraceBonus` | Adds graduated stability-score points (bipod/brace) |
| `GainFireMode((kind: Burst, cone_mult: 1.3, tu_percent: 0.50, shots: 3))` | `FireModeSpec` | ADDS a firing mode to the weapon's selector |
| `ExtraAmmo(8)` | `MagazineSize` | Adds magazine capacity |
| `ReloadTime(0.8)` | `ReloadTimeScale` | SCALES the weapon's `reload_tu` — bidirectional (`< 1.0` faster, `> 1.0` slower). renamed this from the speed-up-only `FastReload` misnomer |
| `Silence` | — | Fits the `Silenced(true)` tag — shots propagate neither suppression nor reaction/reveal |
| `Penetration(2)` | `WeaponPunch` | Adds armor penetration |
| `DamageTypeOverride(Chem)` | `DamageType` | Overrides the emitted damage type (a matchup-wheel re-key) |
| `Damage(2)` | `WeaponDamage` | Adds base damage |
| `Shred(1)` | `WeaponShred` | Adds armor-durability wear per hit |
| `FatalBias(0.5)` | `FatalBias` | Adds to the severity-score addend |
| `Brace` | — | Fits the `Stable(true)` tag — the unconditional brace (distinct from graduated `Stability`) |
| `Shove` | — | Fits the `Shove(true)` tag — knockback on a connecting hit |

### 1e. Fitting an item to a weapon

Reference the item's key from a `.weapon.ron` / `.melee_weapon.ron`:

```ron
    slots:       [(Sight, 1), (Rail, 3)],  // what this weapon OFFERS
    attachments: ["red_dot_sight"],        // what is FITTED (attachment item keys)
```

At battle setup, `resolve_pending_attachments`
(`crates/gdtf_battle_sim/src/equipment/attachments/fit.rs`) resolves each key
against the `AttachmentRegistry` and admits it only into a declared slot with
free capacity (`attachment_fits`, same file); a non-fitting item is cleanly
skipped — never a panic, never an eviction. The admitted effects ride the
spawned weapon entity as `PendingAttachments`
(`crates/gdtf_battle_sim/src/equipment/attachments/apply.rs` consumes it
post-spawn via the `attach_to_weapon` commands extension in
`crates/gdtf_battle_sim/src/equipment/attachments/commands.rs`).

---

## Part 2 — How to extend the attachment model (engineers)

The attachment code splits across two homes:

- **Mechanics** — `crates/gdtf_battle_sim/src/equipment/attachments/`: the
  authoring spec, the key/registry, the slot vocabulary, the fit gate, the
  post-spawn applier, the commands extension.
- **Effect palette** — `crates/gdtf_battle_sim/src/effects/attachments/`: ONE
  self-contained file per effect implementing the `ApplyAttachmentEffect`
  trait, plus the closed `AttachmentEffect` enum in `effect.rs` whose trait
  impl is a purely mechanical delegation match.

Adding a new effect costs exactly:

1. **One per-effect file** in `crates/gdtf_battle_sim/src/effects/attachments/`
   — the payload newtype (if any; `#[serde(transparent)]`), the isolated
   `ApplyYourEffect` behaviour type, its `impl ApplyAttachmentEffect`, and a
   `#[cfg(test)]` unit test. `aim.rs` / `reload_time.rs` are the templates.
2. **One variant** on `AttachmentEffect` (`effect.rs`) + **one delegation arm**
   in its exhaustive match (the compiler forces the arm) + **one `mod` line**
   in the palette's `mod.rs`.
3. **Author it** from an item's `effects:` list (Part 1).

The full recipe, with the design history, lives in the module rustdoc of
`crates/gdtf_battle_sim/src/effects/attachments/effect.rs` — that rustdoc is
the source of truth; this section is the pointer.

---

## Part 3 — Loading and hot-reload

Attachments are one of the generic folder-loaded content families
([content-families.md](content-families.md)): the `AttachmentsFamily` marker
(`crates/gdtf_content_families/src/attachments.rs`) declares folder
`content/attachments` + extension `attachment.ron`, and the game registers it
with one `register_content_family::<AttachmentsFamily>()` line in
`crates/gdtf_app/src/states/load/plugin.rs`. The resolve builds the
`AttachmentRegistry`
(`crates/gdtf_battle_sim/src/equipment/attachments/registry.rs`), keyed by
file stem.

Hot-reload works the same way: run `cargo drun` (the alias enables the
`file_watcher` feature), edit any `*.attachment.ron`, and the generic redrive
rebuilds the registry live, logging an `info!` line naming the reload. The next
battle setup fits the edited items.

Dangling `attachments:` keys and slot mismatches surface on the end-of-`Load`
reference-integrity report — see
[reference-integrity.md](reference-integrity.md).

---

## Part 4 — Verify

- **Suite:** `cargo dtest`. The family's load coverage is
  `crates/gdtf_app/tests/load_attachments.rs` (registry presence + shipped
  stems, value-agnostic); the slot-gated fit path is
  `crates/gdtf_app/tests/load_attachment_fit.rs`; the per-effect fold semantics
  are unit tests inside each palette file
  (`crates/gdtf_battle_sim/src/effects/attachments/`).
- **In game:** `cargo drun` — shipped weapons author attachments (e.g.
  `stub_pistol.weapon.ron` fits `"suppressor"`,
  `chainsword.melee_weapon.ron` fits `"butchers_weight"`); a fitted `Aim`
  optic changes shot grouping, a `Silence`d weapon draws no reaction fire. A
  dangling item key prints on the `Load` reference report instead of crashing.
