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
origin cell, the exit act, walking off the seat and a death freeing the seat
are built. "What exists today" records what they do.

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

**Leaving.** There are two ways out. The exit act returns the ganger to the cell
it entered from; that cell is remembered for as long as it is mounted. Or the
ganger walks off — being mounted refuses no move, and the step that leaves the
seat vacates the emplacement. A walk off the seat leaves by one of the same
rotated entry sides the enter is allowed from, and pays the exit act's TU on top
of the route. The walker stays where it walked, so the exit act applied
afterwards does nothing. While another ganger stands on the remembered cell the
exit act is refused: two gangers are never on one cell, so the occupant stays
mounted and spends nothing, and the Exit button is not offered. The refusal
reads that one cell, so walking off by the other entry sides is still open, at
the route price plus the exit.

**Dying.** A ganger killed in the seat leaves its body there. The corpse stays
on the mount cell and the emplacement goes vacant, so another ganger can climb
in and take the gun: a body slumped over the gun is flavour, one death taking a
heavy weapon out of the fight is not. What a *downed* occupant should do is not
settled.

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
stops a turned mount being entered through its own armour. The walk out is
routed through the same sides for the same reason: a turned mount is not climbed
over its own armour in either direction.

Walking off pays the exit act because leaving the seat is leaving the seat,
whichever way it was asked for. The exit act is still worth having, because the
two differ: it puts the ganger back on the cell it entered from for that cost
alone, while walking off pays the route as well and leaves the ganger wherever
it walked.

## What exists today

`can_enter_emplacement` reads the emplacement's authored entry sides, rotated by
its facing, and allows the enter only from one of those cells; a mount naming no
side cannot be entered at all. Entering writes the ganger's `Position` onto the
emplacement's cell and records on the emplacement the cell the ganger came from;
exiting writes that cell back and drops the record. A vacate with no record — an
emplacement spawned already occupied — leaves the ganger where it is.
`can_exit_emplacement` reads the occupancy grid at that remembered cell, and
refuses while another ganger stands there: no TU is spent, the seat keeps its
occupant, the panel offers no Exit and `battle.cost` prices the act as refused. A
seat holding no record has no cell to test, so it still exits.

A mounted ganger's walk gives the seat up on its first step, written in the
same system and the same tick as the step off the cell, so no tick has a
mounted ganger standing off its seat. A walk that ends before it steps does not
vacate: a dead mover, a reaction shot, a next cell blocked or occupied, and a
pool that cannot pay all stop the walk before the vacate. A living mover stays
mounted; a mover killed before its first step gives the seat up to the death
instead. The walk's vacate sets the state back to vacant, drops the occupant
record and the remembered origin cell, and despawns the mounted weapon. It
writes no `Position`, so the walker keeps the cell it walked to; only the exit
act's vacate writes the origin cell back.
With the occupant record gone, the exit act finds no emplacement the ganger
holds and does nothing. Its dispatch is ordered after the walk step, so an exit
asked for in the same frame as the move reads the seat after that vacate. It
charges nothing, and the dismount is paid once, on the step.

Two enter requests, or two exit requests, written in the same frame charge once.
Each dispatch remembers the seat it has already written this run and gates the
next request on that rather than on the queried state, so a repeat enter reads
the seat as occupied and a repeat exit reads it as vacant. Both are refused, and
nothing is spent on them.

`clear_seat_on_death` (`terrain/emplacement/death.rs`) reads the gangers whose
life state changed to dead this frame and gives up the seat each one rides,
through the same `clear_seat` a walk off the seat uses: the state goes back to
vacant, the occupant record and the remembered origin cell are dropped, and the
mounted weapon is despawned. It writes no `Position`, so the body is left on the
mount cell. It runs after `apply_emplacement_toggle`, after `resolve_on_death`
and after `sync_dead_gangers` — so the death fans its effect while the mount is
still there, and the grid has already released the corpse's slot by the time the
seat frees. Writing the corpse's cell back would re-claim that slot with nothing
left to release it, which is why the system touches no position. A mounted
weapon carries the on-death effect its spec authors, the way a carried one
does, so a gunner killed at the mount fires the mount's effect rather than the
gun on their back. Only death is handled: a downed occupant keeps its seat and
its grid slot both.

The route out is planned from the seat's rotated entry cells: the first step may
land only on one of them, and the rest of the route carries on from there. A
destination no entry side reaches is unreachable, and a pool that cannot cover
the route plus the exit is refused for cost. The exit TU rides on the first step
of the walk, so the steps sum to the quoted total and the dismount is charged
once; a walk that never takes that step pays neither the step nor the exit. The
quote, the path preview and the committed move read the same helpers, so they
agree on both the route and the price.

While it is mounted the ganger is the occupant of that cell like any other. The
toggle writes no occupancy band of its own, so the band at the emplacement's
cell is the mounted ganger's own stance silhouette, published by the same system
that publishes every other ganger's. A shot at that cell is aimed at the
emplacement's authored cover height rather than at the ganger's silhouette,
because the cover ledger's entry is read before the occupant band.

Which ganger rides which emplacement is a Bevy relationship — `MountedBy` on the
emplacement, `Mounted` on the ganger, kept in step by the engine. It is
one-to-one: a ganger cannot be mounted in two emplacements at once.

Inspecting the emplacement's cell reports the piece as an emplacement, whether
it is manned, and the weapon it mounts, alongside the cover stats the ledger
holds for it and the card for a ganger on it the squad can see.

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
- Code sites: these live in `crates/gdtf_battle_sim` unless named otherwise.
  The def's `entry_sides` and the `rotated_entry_sides` rotation are in
  `terrain/def/kind.rs`. The `EmplacementEntrySides` and `EmplacementFacing`
  components battle setup puts on the spawned piece, and the `EnteredFrom`
  record of the origin cell, are in `terrain/emplacement/state.rs`. The
  `MountedBy` / `Mounted` relationship is in
  `terrain/emplacement/relationship.rs`. `can_enter_emplacement`,
  `can_exit_emplacement` and their dispatches are in
  `acts/enter_emplacement.rs`, and the `PendingStates` map the dispatches read
  their own writes back from is in `acts/pending_state.rs`; `wire_acts`
  (`acts/plugin/acts.rs`) orders `dispatch_exit_emplacement` after
  `advance_walk`. `apply_emplacement_toggle`, which writes both `Position`
  moves, is in `terrain/emplacement/toggle.rs`.
  `clear_seat` (`terrain/emplacement/vacate.rs`) is the vacate the toggle, a
  walk's first step and `clear_seat_on_death` (`terrain/emplacement/death.rs`)
  share, and `emplacement_entry_cells`
  (`terrain/emplacement/entry.rs`) is the rotated entry set the enter gate and
  the route out share.
  `dismount_surcharge` and `seat_departure` are in `acts/movement/mount.rs`; the
  surcharge is added by `move_tu_cost` and `move_step_tu_costs` in
  `acts/movement/cost.rs`, and the departure reaches the search as a `Departure`
  (`perception/pathfinder/departure.rs`) handed to `find_path_leaving`
  (`perception/pathfinder/search.rs`). The path preview
  (`crates/gdtf_battle_input/src/pointer/selection/path_preview.rs`) and the
  `battle.cost` walk quote
  (`crates/gdtf_app/src/dev/net_qa/commands/read/battle_cost/price/walk.rs`)
  call the same helpers. The two callers of the exit gate outside the sim are
  `offer_exit_emplacement`
  (`crates/gdtf_app/src/states/running/game/battlescape/contextual_panel/acts/exit_emplacement.rs`)
  and `exit_quote`
  (`crates/gdtf_app/src/dev/net_qa/commands/read/battle_cost/price/reach.rs`);
  both hand it the `OccupancyGrid`.
- Source: owner rulings, given directly in conversation, 2026-08-14.
