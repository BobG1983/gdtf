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

Status: decided, not built. "What exists today" below records the state this
replaces, so a reader can tell canon from code.

## The claim

Which art views a def owes follows from its kind and its properties:

| What the def is | Views it owes |
|---|---|
| Wall kind | its edges and its corners |
| Cover kind | its facings |
| Emplacement kind | its facings |
| Openable tag — a door | its facings, plus open and shut |
| Stair link — a staircase | the view from below and the view from above — one def, not two |

There is no `Door` kind and no `Staircase` kind. The sim has four terrain
kinds — wall, cover, slab, emplacement — and a door is an openable tag with an
open state, a staircase a vertical link. Each row above is keyed off the marker
that actually exists, not off an invented kind.

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

The presenter derives a role for each cell and, when a cell has no authored
piece, resolves art by the role's own name — so roughly twenty sprite records
are claimed by render code rather than by any content. A test asserts those
files exist. The missing-tile texture is already generated at runtime; it is
simply reached more often than it should be. The `on_death` list the claim
names is built on terrain defs and weapon specs both; `leaves_behind` is not.

## Consequence for the editor

Art becomes referenced by defs like any other content, so deleting a sprite is
an ordinary delete: check what uses it, and if nothing does, delete it cleanly.
Only when a def does use it is a replacement demanded. It stops being a special
case in the delete surface.

`leaves_behind` adds a second reference of the same ordinary kind — one piece
naming another — caught by the same check and fixed the same way. Chains make
that graph deeper, not different.

This must land before sprite deletion is built. Until it does, those names are
claimed by render code and no replacement can satisfy them.

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
- Code sites: the sim's terrain kinds and the `on_death` field live in
  `crates/gdtf_battle_sim`; role-name art resolution lives in
  `crates/gdtf_battle_presenter`; the authoring forms live in
  `crates/gdtf_content_editor`. **TBD (Bevy):** `leaves_behind`, the per-def
  view sets, emplacement facing, and the per-def completeness check are not
  built.
- Source: owner rulings, given directly in conversation, 2026-08-14.
