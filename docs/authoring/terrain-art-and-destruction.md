---
title: Terrain art ownership and destruction
kind: canon
status: Accepted
date: 2026-08-14
pillars: [1, 5]
---

# Terrain art ownership and destruction

**A terrain def owns its own art in every state it can be in, and says what it
leaves behind when it dies; the renderer owns no content.**

Status: `leaves_behind`, the per-def view sets and the presenter's view
resolution are built. A wall's corner views are authored and unreachable: the
resolver picks an edge view from the piece's own facing, and nothing scans a
cell's neighbours. "What exists today" below records what the code does, so a
reader can tell canon from code.

## The claim

Which art views a def owes follows from its kind and its properties:

| What the def is | Views it owes |
|---|---|
| Wall kind | its edges and its corners |
| Cover kind | its facings |
| Emplacement kind | its facings |
| Slab kind | one view |
| Openable tag — a door | shut and open for each of the four facings, and no plain facing row |
| `Stair` tag — a staircase | the view from below and the view from above for each of the four facings, on one def, not two |

A tag row wins over a kind row, and `Openable` wins over `Stair`. A door
authors `sim_kind: Wall` and a staircase authors `sim_kind: Slab`, so reading
the kind first would give a door a wall's edges and corners and a staircase one
floor view.

There is no `Door` kind and no `Staircase` kind. The sim has four terrain
kinds — wall, cover, slab, emplacement — and a door is an openable tag with an
open state. The stair row keys off `TerrainTag::Stair`, added on the def,
because the `VerticalLink` carrying `LinkKind::Stair`
(`crates/gdtf_battle_sim/src/terrain/vertical/links.rs`) lives on the link graph
and not on the def, so the def had nothing to key off.

Nothing owes a "destroyed" view. A def carries `leaves_behind`, which is one of:

- another def — the wreckage is an ordinary piece with its own stats and art,
  and may itself leave something behind;
- a sprite with no mechanics — decoration, a scorch mark;
- nothing, the default, so the theme's floor shows through.

`on_death` is a **list** of effects, on terrain and weapons both: a piece can
explode and leave a field and leave rubble. What happens and what remains are
different questions, so they are different fields.

An occupied emplacement is not terrain art. The ganger draws itself in place.

Emplacements carry a facing in the sim. The mounted weapon's arc respects it,
and turning is an act with a cost like any other.

Themes are unchanged: a theme lists its pieces and names a default floor, and
owes nothing per state. A theme with no doors has no door def, so there is
nothing to draw and nothing missing.

The missing-tile texture already works this way: generated at runtime, magenta,
no content file. What changes is when it appears — only where a cell has no
piece and no theme default. That is a genuine authoring gap, and the only thing
it ever signals.

## Why

The renderer needs art for states no authored piece describes: a door standing
open, a floor slab smashed, an emplacement with someone in it, a wall running
east-west. The question is who supplies it.

Naming that art from render code makes the renderer a content owner. Three
things follow, all bad. Every theme shares one look for the things that appear
in every single map — doors, stairs, rubble — so themes stop distinguishing
places. The art becomes unreferenced by any content, so the editor cannot apply
its ordinary rule when an author deletes it. And a piece cannot describe its own
destruction, so every wrecked thing leaves the same pile.

Putting the art on the def fixes all three, and putting *destruction* on the def
as `leaves_behind` turns "what does destruction leave" from a global rule into a
per-piece authoring decision — a hab wall collapses into low cover you can still
hide behind, a fungus stack simply goes.

## What exists today

The presenter resolves a drawn view from the piece standing at the cell: the
def it was placed from, the facing it carries and its open state. Battle setup
seeds a floor piece from the situation's default floor across the whole extent
the presenter draws, so the missing-tile marker shows only on a storey-0 cell
whose floor def does not resolve. Occupied-emplacement art is gone and the
occupant draws itself on the seat. A ladder endpoint is the one sprite record
render code still names, taken from the link kind rather than from a def.

`draw_static_battlefield` stamps each tile with the view the piece's own def
names for its facing and its open state, resolved through `view_key_for`, and
`stamp_destroyed_cell` resolves a smashed cell's successor the same way. A def
whose `leaves_behind` names a sprite stands that sprite in the cell instead, and
both reads draw it ahead of any piece still standing there. The `on_death` list
the claim names is built on terrain defs and weapon specs both, and
`leaves_behind` is built on terrain defs.

## Consequence for the editor

Art becomes referenced by defs like any other content, so deleting a sprite is
an ordinary delete: check what uses it, and if nothing does, delete it cleanly.
Only when a def does use it is a replacement demanded. It stops being a special
case in the delete surface.

`leaves_behind` adds a second reference of the same ordinary kind — one piece
naming another — caught by the same check and fixed the same way. Chains make
that graph deeper, not different.

Sprite deletion is not built. The one name still claimed by render code is
`ladder`, which a link endpoint draws from its link kind, and `door`,
`stair_up` and `stair_down` are named by tests and by no def.

## Left open

Damage sequences. One view per stage for now, to keep the art bill low; the
slot can become a list without disturbing anything else.

## Links

- Pillars: [1 — situation generator](../pillars/1-situation-generator.md),
  [5 — readability over fidelity](../pillars/5-readability-over-fidelity.md).
- Litmus ([litmus-tests.md](../litmus-tests.md)): passes 4 — destruction and
  state become *more* legible, because a smashed hab wall stops looking like a
  smashed fungus stack. Passes 1 — themes gain real visual identity in the
  pieces every generated map contains.
- Related guides: [terrain-authoring.md](terrain-authoring.md),
  [sprite-defs.md](sprite-defs.md),
  [on-death-authoring.md](on-death-authoring.md),
  [reference-integrity.md](reference-integrity.md).
- Code sites: the sim's terrain kinds, the `on_death` field and the
  `leaves_behind` field live in `crates/gdtf_battle_sim/src/terrain/def/`, and
  `crates/gdtf_battle_sim/src/terrain/successor/` acts on it when a piece is
  destroyed; `TerrainView`, `TerrainViewArt`, `TerrainViews` and `owed_views`
  live in `crates/gdtf_battle_sim/src/terrain/def/views/`, and the per-def
  completeness check in
  `crates/gdtf_content_families/src/validate/terrain_views.rs`; per-def view
  resolution lives in
  `crates/gdtf_battle_presenter/src/render/terrain/view_resolve.rs` and the
  restamp that drives it in `view_restamp.rs`; the floor seeding lives in
  `crates/gdtf_battle_sim/src/lifecycle/situation/setup/seed_floor.rs`; the
  authoring forms live in `crates/gdtf_editor`. **TBD (Bevy):** the
  mounted weapon's arc reading the mount's facing, and turning a mount as an
  act, are not built.
- Source: owner rulings, given directly in conversation, 2026-08-14.
