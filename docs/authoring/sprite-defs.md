# Sprite Defs Authoring Guide

How to author SPRITE DEFINITIONS — the per-file `.spritedef.ron` catalog of
drawable sprites that terrain defs (and, later, other content) reference by
NAME (GTW-663; the GTW-600 data-model ruling). A sprite def says *where a
sprite's pixels come from* (a standalone image, or a `{sheet, rect}` region of
a sprite sheet), *where it touches the ground* (the anchor/pivot), and
optionally *how it faces and animates*.

**Status today:** the defs are the authoritative sprite data model AND the
authoritative RESOLUTION: since **GTW-665** the battle renderer and the
content editor both resolve a terrain `graphic_name` to its sprite def
(texture + rect + anchor) through this registry — the legacy terrain role
table (`tile_roles.spritedef.ron` → `TileRoles`) is retired and deleted. The
editor authoring mode is **GTW-664**; restamping shipped content is
**GTW-666**.

---

## Part 1 — Where the files live (and the naming collision to know about)

Sprite defs live under `assets/content/sprites/` — one file per sprite, flat.
The file is named `<name>.spritedef.ron`; the FILE STEM is the sprite's
registry name (`floor.spritedef.ron` → name `"floor"`), exactly the string a
terrain def's `graphic_name:` references.

Do NOT confuse this family with the OLD role tables under `assets/sprites/`
(`character_roles.spritedef.ron`, `effect_roles.spritedef.ron`) — same
suffix, different format and folder. Those are single-file role→atlas-index
tables the ACTOR/FX draws still use; the TERRAIN table
(`tile_roles.spritedef.ron`) was retired and deleted when GTW-665 swapped
terrain resolution onto this family.

## Part 2 — The `.spritedef.ron` schema — by example

From the seeded `assets/content/sprites/floor.spritedef.ron` (magnitudes are
shape examples, never pinned values):

```ron
(
    source: Sheet(
        sheet: "sprites/alt_tileset_terrain.png",  // the presenter's terrain sheet
        rect: (x: 96, y: 0, w: 16, h: 16),  // one 16x16 sheet cell
    ),
    anchor: (x: 8, y: 8),  // ground-contact/pivot, sprite-local px from the top-left
)
```

The full shape, including the two optional fields no seed authors yet:

```ron
(
    source: File("sprites/lone_crate.png"),  // OR a standalone image file
    anchor: (x: 8, y: 15),                   // e.g. bottom-center ground contact
    facings: Some({                          // OPTIONAL per-facing source overrides
        North: File("sprites/lone_crate_n.png"),
        East:  Sheet(sheet: "sprites/crates.png", rect: (x: 16, y: 0, w: 16, h: 16)),
    }),
    animation: Some((                        // OPTIONAL animation
        fps: 4.0,                            // playback rate, frames/second
        frames: [                            // ordered frame sources, looped
            File("sprites/lone_crate.png"),
            File("sprites/lone_crate_2.png"),
        ],
    )),
)
```

**Field reference** (`SpriteDef`,
`crates/gdtf_content_families/src/sprites/def.rs` — the sources in
`crates/gdtf_content_families/src/sprites/source.rs`):

| Field | Rust type | RON form | Notes |
|-------|-----------|----------|-------|
| *(name)* | `SpriteName` | *the file stem* | NOT a payload field — the member's file stem is its registry key (stem-keyed family) |
| `source` | `SpriteSource` | `File("<path>")` \| `Sheet(sheet: "<path>", rect: (x:…, y:…, w:…, h:…))` | Where the pixels come from; paths are asset-root-relative; the rect is sheet pixels from the sheet's top-left |
| `anchor` | `SpriteAnchor` | `(x: …, y: …)` | Ground-contact/pivot, sprite-LOCAL pixels from the sprite's top-left. The seeds author `(8, 8)` — the CURRENT implicit anchor (the renderer centers each 16×16 tile on its cell) |
| `facings` | `Option<SpriteFacings>` | `Some({ North: <source>, … })`, omit for none | Per-facing source OVERRIDES over the closed 4-facing enum (`North`/`East`/`South`/`West`); an absent facing falls back to `source` |
| `animation` | `Option<SpriteAnimation>` | `Some((fps: …, frames: […]))`, omit for none | An ordered frame-source sequence played at `fps` |

## Part 3 — How it registers, resolves, and hot-reloads

The family rides the generic content-family seam
([content-families.md](content-families.md)) — one
`register_content_family::<SpriteDefsFamily>()` line per host (the game's
`Load` plugin AND the content editor's `Load` pass) buys the dedicated-
extension loader, per-file salvage (a malformed def fails ALONE, as a
`malformed file:` finding), the fail-closed empty registry, the headless
fallback, and the live hot-reload redrive: edit a `.spritedef.ron` under
`cargo drun` and the whole `SpriteDefRegistry` rebuilds in place.

The registry is `SpriteDefRegistry`
(`crates/gdtf_content_families/src/sprites/registry.rs`). Unlike every other
family its spec/registry types live in the GLUE crate, not the sim: sprite
data is presentation-side, and the render-free sim cannot own it.

**The `graphic_name` foreign key.** A terrain def's `graphic_name:` is a
foreign key by NAME into this registry — see
[terrain-authoring.md](terrain-authoring.md) §1d. The reference-integrity
pass ([reference-integrity.md](reference-integrity.md)) checks the edge in
BOTH hosts: a `graphic_name` that resolves no sprite def lands a
`DanglingRef` finding against `SpriteDefRegistry` on the consolidated report,
at the end of the game's `Load` and live at editor authoring time (the watch
set re-arms the pass when the sprite registry rebuilds).

## Part 4 — The seeded catalog (GTW-663 C2) and how it resolves (GTW-665)

GTW-663 seeded one def per name reachable as a `graphic_name` today — the 20
`TileRole` keys — each mechanically derived from the then-live role table:
sheet = the presenter's terrain sheet, rect = the role's atlas index unpacked
on the sheet's 16-column/16-px grid, anchor = the implicit CENTER anchor
`(8, 8)` (the renderer draws each tile as a unit quad centered on its cell —
`crates/gdtf_battle_presenter/src/render/terrain/static_draw.rs`).

Since GTW-665 the defs ARE what renders: the presenter's ONE resolution
(`resolve_sprite`,
`crates/gdtf_battle_presenter/src/render/terrain/resolve.rs`) looks a
`graphic_name` up in the registry by NAME; `source` picks the pixels (a
`Sheet` source's rect, or a `File` image), and `anchor` places the sprite (the
authored anchor point sits ON the cell position — a center anchor reproduces
the old centered draw exactly). The content editor's palette / preview
thumbnails consume the SAME resolution — one resolution, two consumers. The
GTW-663 derivation-truth pin retired with the table, per its own retirement
note: there is no second artifact left to drift from.

A `graphic_name` that resolves NO def draws the LOUD magenta missing-sprite
marker (never a panic, never an invisible tile) and warns naming the key and
the cell — possible mid-authoring, since the integrity edge warns without
blocking load.

## Part 5 — Verify

- **Suite:** `cargo dtest` — the family binds the generic load suite
  (`crates/gdtf_app/tests/load_sprites.rs`), the schema
  pins live in-crate (`crates/gdtf_content_families/src/sprites/test/`), the
  def-driven draw / off-center-anchor / missing-marker behavior is pinned in
  `crates/gdtf_battle_presenter/tests/terrain_draw/` and
  `crates/gdtf_battle_presenter/tests/terrain_missing_sprite.rs`, and
  the dangling-`graphic_name` finding is pinned in both hosts
  (`crates/gdtf_app/tests/load_ref_integrity.rs`,
  `crates/gdtf_content_editor/tests/authoring_validation/sprites.rs`).
- **Hot-reload:** `cargo drun`, edit a `.spritedef.ron`, watch the
  "hot-reload: rebuilt … from content/sprites" info line.
- **Dangling key:** author a `graphic_name` with no matching sprite def and
  watch the `DanglingRef … SpriteDefRegistry` finding on the end-of-`Load`
  report (game) or the live editor report (authoring time) — and the magenta
  missing-sprite marker at any cell that draws it.
