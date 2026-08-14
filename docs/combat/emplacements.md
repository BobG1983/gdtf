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

Status: decided, not built. "What exists today" records the state this replaces.

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

`can_enter_emplacement` requires 8-adjacency, and mounting leaves the ganger on
its own cell. Only the occupancy band is set at the emplacement's cell, so an
occupied emplacement is indistinguishable from an empty one on screen, and the
ganger keeps whatever cover it was already in. There is no entry-side data, no
remembered origin cell, and no exit act.

The first implementation was wrong. This replaces it rather than extending it.

## Links

- Pillars: [4 — permanent stakes](../pillars/4-permanent-stakes.md),
  [5 — readability over fidelity](../pillars/5-readability-over-fidelity.md).
- Litmus ([litmus-tests.md](../litmus-tests.md)): passes 3 — mounting becomes a
  decision with a real downside, where today it is a free upgrade. Passes 4 — an
  occupied mount becomes legible, which it is not now.
- Related: [combat.md](combat.md) for cover and Time Units,
  [resolution.md](resolution.md) for the firing arc,
  [../authoring/terrain-authoring.md](../authoring/terrain-authoring.md) for the
  def the entry sides are authored on.
- Code sites: `can_enter_emplacement` and the act live in
  `crates/gdtf_battle_sim`. **TBD (Bevy):** entry sides, the remembered origin
  cell, the exit act, and the facing they rotate with are not built.
- Source: owner rulings, given directly in conversation, 2026-08-14. Ticket:
  GTW-1176.
