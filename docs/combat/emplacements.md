---
title: Emplacements
kind: canon
status: Accepted
date: 2026-08-14
pillars: [4, 5]
---

# Emplacements

**A ganger enters an emplacement and occupies its cell; it does not stand beside
it and reach in.**

Status: the entry gate, the move onto the emplacement's cell, the remembered
origin cell and the exit act are built. "What exists today" records what they
do.

## The claim

An emplacement is a mounted position — a heavy weapon on a pintle, a firing
slit. Using one means being in it.

**Entering.** An emplacement def names the sides it can be entered from: up,
down, left, right. Those sides rotate with the emplacement's facing, so a mount
turned to face east has its entry sides turned with it. A ganger standing on one
of those cells is offered the act to enter, and entering moves it onto the
emplacement's own cell.

**Occupying.** A mounted ganger is on the emplacement's cell and is in high
cover. It is not adjacent to the mount; it is in it.

**Leaving.** A mounted ganger is offered the act to exit, which returns it to
the cell it entered from. That cell is remembered for as long as it is mounted.

**Emplacements nothing can enter.** A def naming no entry side cannot be
entered, and offers no act. That is how a mount is authored as scenery, or for
something that was never going to climb into it.

**Blocking.** An emplacement blocks movement and line of sight. Treating it as
cover may give both without new code; check before writing either.

## Why

A mount that is used from the next cell along is not a mount. It reads wrong —
the tile looks identical whether or not someone is manning it — and it plays
wrong, because the ganger keeps its own cover, its own facing and its own
exposure while getting the mount's weapon for free. Occupying the cell makes the
trade real: you take the mount's protection and the mount's arc, and you give up
being anywhere else.

Naming the entry sides in the def, rather than deriving them, is what lets a
firing slit differ from an open pintle. Rotating them with the facing is what
stops a turned mount being entered through its own armour.

## What exists today

`can_enter_emplacement` reads the emplacement's authored entry sides, rotated by
its facing, and allows the enter only from one of those cells; a mount naming no
side cannot be entered at all. Entering writes the ganger's `Position` onto the
emplacement's cell and records on the emplacement the cell the ganger came from;
exiting writes that cell back and drops the record. A vacate with no record — an
emplacement spawned already occupied — leaves the ganger where it is.

While it is mounted the ganger is the occupant of that cell like any other. The
toggle writes no occupancy band of its own, so the band at the emplacement's
cell is the mounted ganger's own stance silhouette, published by the same system
that publishes every other ganger's. A shot at that cell is aimed at the
emplacement's authored cover height rather than at the ganger's silhouette,
because the cover ledger's entry is read before the occupant band.

Which ganger rides which emplacement is a Bevy relationship — `MountedBy` on the
emplacement, `Mounted` on the ganger, kept in step by the engine. It is
one-to-one: a ganger cannot be mounted in two emplacements at once.

## Links

- Pillars: [4 — permanent stakes](../pillars/4-permanent-stakes.md),
  [5 — readability over fidelity](../pillars/5-readability-over-fidelity.md).
- Litmus ([litmus-tests.md](../litmus-tests.md)): passes 3 — mounting is a
  decision with a real downside, not a free upgrade. Passes 4 — an occupied
  mount is legible, because the ganger is standing on it.
- Related: [combat.md](combat.md) for cover and Time Units,
  [resolution.md](resolution.md) for the firing arc,
  [../authoring/terrain-authoring.md](../authoring/terrain-authoring.md) for the
  def the entry sides are authored on.
- Code sites: all of these live in `crates/gdtf_battle_sim`. The def's
  `entry_sides` and the `rotated_entry_sides` rotation are in
  `terrain/def/kind.rs`. The `EmplacementEntrySides` and `EmplacementFacing`
  components battle setup puts on the spawned piece, and the `EnteredFrom`
  record of the origin cell, are in `terrain/emplacement/state.rs`. The
  `MountedBy` / `Mounted` relationship is in
  `terrain/emplacement/relationship.rs`. `can_enter_emplacement`,
  `can_exit_emplacement` and their dispatches are in
  `acts/enter_emplacement.rs`, and `apply_emplacement_toggle`, which writes both
  `Position` moves, is in `terrain/emplacement/toggle.rs`.
- Source: owner rulings, given directly in conversation, 2026-08-14.
