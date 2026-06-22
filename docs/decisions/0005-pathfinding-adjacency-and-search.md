---
name: "ADR 0005: Grid pathfinding — adjacency, search, and vertical stitching"
description: Resolve GTW-12's parked pathfinding forks — adjacency/diagonal policy (user-ratified) and the deterministic weighted-search core that serves both point-to-point routing and the reachable-range overlay over per-storey grids stitched by VerticalLinkGraph.
---

# 0005. Grid pathfinding — adjacency, search, and vertical stitching

## Status

`Proposed` — 2026-06-21, driven by [GTW-12](https://linear.app/robert-gardner/issue/GTW-12).

This ADR resolves the two design forks that GTW-12 was parked on. **OQ-2 (search
algorithm + deterministic tie-break)** is an engineering-determined call and is
recommended with confidence below. **OQ-1 (adjacency / diagonal / corner-cutting)**
is a tactical-feel call the user parked deliberately; the recommendation here is
**tentative and awaits user ratification** — it may be ratified or overridden
without disturbing OQ-2.

## Context

GDTF's core loop carries a scarred roster from fight to fight, and the fights are
turn-based tactics over a **60×60×8 cubic-voxel grid** ([battle-space.md](../combat/battle-space.md)):
x/y are ground-plane cells, z is a storey level, one sim unit = one cell = one
level, and the canonical identity is `CellLevel` wrapping `IVec3` (`(x, y, level)`)
(`crates/gdtf_battle_sim/src/metric/coords.rs`). There are no pixels in the model.

GTW-12 owes the sim two pathfinding deliverables:

1. **Point-to-point routing** — given a start and a goal `CellLevel`, the cheapest
   legal route across one or more storeys.
2. **A reachable-range overlay** — every `CellLevel` reachable within a TU budget
   (the move-preview / highlight footprint).

Several layers underneath are **already built and fixed**, and this ADR does **not**
redesign them:

- **The movement economy is canon** ([visibility.md `## Pay-per-step TUs`](../combat/visibility.md),
  referred to below as §48 — the in-repo shorthand for that section, matching the
  code comment in `tuning/economy.rs`). TU charges land **per step**: `sync_ganger_cell`
  charges the entered cell's terrain `move_cost`; a vertical-link hop charges the flat
  `link_tu` **instead of** terrain (`crates/gdtf_battle_sim/src/tuning/economy.rs`). The
  **move-commit step gates full-route affordability once, up front**; per-step charges then
  always succeed (strictly turn-based). Interruptions are arithmetic-free — *charged = ground
  covered*; the old spend-plus-refund economy is deleted. **Pathfinding must honour
  this economy, not invent its own.**
- **Per-terrain weighted cost is fixed**: `MoveCost(u8)` per `TerrainKind`, defaults
  `open=4, cover=6, wall=8` (`tuning/economy.rs`). **The minimum positive move cost
  on the grid is 4** — load-bearing for the search heuristic below.
- **The OccupancyGrid** is the authoritative blocked/walkable surface, change-driven
  (maintained in place, never rebuilt per shot — the GTW-6/GTW-12 ruling)
  (`crates/gdtf_battle_sim/src/occupancy/grid.rs`).
- **The VerticalLinkGraph** indexes authored stair/ladder links, validated at setup,
  bidirectional unless one-way, queryable via `links_from(origin)`
  (`crates/gdtf_battle_sim/src/vertical/graph.rs`). It is explicitly **existence-only
  today — "there is no traversal / pathfinding / movement cost here (that is GTW-12)."**
- **Determinism is a hard constraint**: the sim is seeded-replayable; equal-cost
  choices must resolve in a fixed total order or two replays diverge. Movement reads
  consume no RNG. The established precedent is the `auto_select` cell-key sort by
  `(z, y, x)` — **not** entity-ID order (the GTW-255 `auto_select_first_player_ganger`
  precedent, `cell_order_key`).

What is **not yet decided**, and why a decision is needed now:

- **OQ-1 — adjacency model**: 4-connected (orthogonal only) vs 8-connected (with
  diagonals), plus, if 8-connected, the **diagonal cost** and the **corner-cutting
  legality** policy. This shapes the neighbour-enumeration that the search graph is
  built from, and it is fundamentally about how movement *feels*.
- **OQ-2 — search algorithm + deterministic tie-break**: which search powers *both*
  deliverables, and how equal-cost frontier expansions are pinned for byte-equal
  replay.

There is no pathfinding code today; `move_ganger` steps a single `dest: CellLevel`
per call (`crates/gdtf_battle_sim/src/move_acts/verb.rs`). GTW-12 introduces the
search and the neighbour model; it does not touch the economy.

## Decision

### OQ-1 — Adjacency, diagonal cost, corner-cutting *(tactical-feel — tentative, awaiting user ratification)*

This is a design/feel call, not an engineering-correctness one, and the user parked
it on purpose. We present both honestly and recommend tentatively.

**4-connected (orthogonal only)**

- For: simplest to build; clean axis-aligned cover facings (4 faces) that are easy to
  reason about; no diagonal-cost question; no corner-cutting rule needed.
- Against: blocky, staircase routes around obstacles; coarse flanking angles; under-uses
  the existing 8-way `Direction` compass (`crates/gdtf_battle_sim/src/ganger/direction.rs`,
  N/NE/E/SE/S/SW/W/NW with `forward_step` and `steps_to`).

**8-connected (orthogonal + diagonal)**

- For: shorter, more natural routes and richer flanking; consistent with the 8-way
  facing model already in the sim; matches the genre target — **XCOM: EU/2012 uses
  true 8-directional tile movement with diagonals weighted heavier** (12 mobility ≈ 7
  tiles straight vs ~5 diagonal). That is direct precedent for the look GDTF chases.
- Against: obligates **both** a diagonal-cost rule **and** a corner-cutting rule to
  avoid two well-known exploits (free √2 ≈ 41% extra distance per diagonal TU;
  "phasing" a diagonal through the corner where two walls meet); 8 cover facings
  instead of 4.

**Necromunda offers no grid precedent either way** — tabletop Necromunda is
inch-based free measurement, gridless; if anything its any-angle feel argues for the
*continuous* approximation that 8-connected gives, not for 4.

If 8-connected is ratified, two sub-policies are mandatory (they are exploit-fixes,
not taste):

- **Diagonal cost = octile (≈ √2 × orthogonal).** With integer TU we approximate the
  ratio (e.g. orthogonal=4 / diagonal=6, ratio 1.5, reusing the existing min-cost-4
  scale; a tighter `≈1.41×` is possible). Flat same-cost diagonals are rejected — they
  make diagonal dashes strictly best.
- **No corner-cutting between two edge-adjacent blocked cells** (the
  `AT_LEAST_ONE_WALKABLE` rule): a diagonal step is illegal if both shared-edge
  orthogonal neighbours are blocked. Matters on cover-heavy grimdark maps.

> **Recommendation (tentative): adopt 8-connected with octile diagonal cost and the
> no-corner-cutting rule.** Rationale: it matches the XCOM precedent GDTF is chasing
> and reuses the load-bearing `Direction` compass. **This is the user's call to ratify
> or override.** Critically, OQ-2 below is **adjacency-agnostic** — it works identically
> for 4- or 8-connected, so this choice locks nothing in the search/tie-break.

### OQ-2 — Search algorithm + deterministic tie-break *(engineering-determined — recommended)*

We will implement **one weighted uniform-cost search core (Dijkstra)** over the
weighted, directed, multi-storey graph, and use it for **both** deliverables:

- **Reachable-range overlay**: Dijkstra *is* the bounded distance-field flood — seed
  the frontier at the mover's `CellLevel` at cost 0 and stop expanding when the
  accumulated cost exceeds the TU budget. A heuristic is meaningless here (no single
  goal), so this is Dijkstra's native shape.
- **Point-to-point routing**: layer an **admissible heuristic** on the *same*
  priority-queue core to focus the search toward the goal — i.e. A* = Dijkstra + `h`.
  The admissible heuristic is the **planar step-distance × the minimum move cost
  (4)**: `h = chebyshev_xy(a, b) * 4` for 8-connected (`manhattan_xy × 4` for
  4-connected). Scaling by the *minimum* per-step cost guarantees `h` never
  overestimates. The planar heuristic ignores z and link costs entirely — it
  contributes 0 for a storey change, which can never overestimate, so it **stays
  admissible across storeys** (it merely weakens toward Dijkstra when the goal is
  mostly above/below — safe, just slower). A tighter `+ |Δz| × min_link_cost` term is
  a future optimization, not a correctness requirement.

**BFS is rejected**: it counts edges and is correct only on uniform costs; our costs
are 4/6/8 plus per-link `link_tu`, so BFS returns wrong-cost paths.

**Deterministic tie-break (required for byte-equal replay).** A binary heap is not
stable on equal keys, and `HashMap` iteration order (e.g. over `links_from`) is not
deterministic in Rust. We therefore order the priority queue by the tuple:

```
(total_cost, [optional straightness/larger-g tier], cell_key)
```

where **`cell_key` is the canonical `(z, y, x)` ordering of `CellLevel`** — the same
total order `auto_select` already uses (the GTW-255 `cell_order_key` precedent). The
cell key as the **final** component gives a data-only total order that depends on map
data alone, not heap internals, insertion timing, or `HashMap` iteration. An optional
straightness/larger-`g` tier may sit *above* the cell key for nicer-looking paths, but
never *instead* of it. **Neighbour enumeration must also be sorted deterministically**
(planar neighbours and `links_from` results iterated in a fixed order), or any
intermediate tier silently reintroduces nondeterminism.

### Vertical stitching *(confirm §48 — not redesigned)*

The single multi-storey search treats a vertical link as just another outgoing edge:
at the current `CellLevel`, enumerate planar neighbours (per OQ-1) **and**
`VerticalLinkGraph::links_from(current)`. A link edge goes to `link.to`, costs the
flat **`link_tu` instead of** the destination terrain cost (per §48), and respects the
link's one-way flag. One search explores all 8 storeys; the per-storey grids are
stitched solely by the link graph — there is no separate ground-then-vertical pass.
The economy itself is untouched: the search *plans* the route and totals its cost; the
existing per-step writers *charge* it at commit, and the **move-commit affordability
gate stays the single up-front check** (§48).

## Consequences

- **Both GTW-12 deliverables share one core.** The reachable-range overlay and
  point-to-point routing differ only by `h` (≡ 0 for the flood, `chebyshev × 4` for
  routing) — one priority-queue, one relaxation, one tie-break to test and maintain.
- **The economy contract is preserved, not duplicated.** Pathfinding reads
  terrain/`link_tu` costs and totals them; it never charges TU and never adds refunds.
  The move-commit gate remains the only affordability check (§48).
- **Replay stays byte-equal** as long as the `(cost, …, (z,y,x) cell_key)` order and
  sorted neighbour enumeration are honoured. This is a **testable invariant** — a
  determinism regression (same map + budget → identical route and identical reachable
  set) should ship with the search-core child ticket.
- **Determinism note must be enforced in code**: keyed/cell-based lookups only; no
  `Entity`-ID-order iteration, no raw `HashMap`-order iteration in the hot loop.
- **If 8-connected is ratified**, the build owes the octile cost *and* the
  no-corner-cutting rule together; shipping diagonals without both reopens the two
  exploits. The cover-facing model (4 vs 8 faces) inherits the choice.
- **A* is deferrable.** Shipping Dijkstra-only (h ≡ 0) for routing is correct, just
  slower; the heuristic is a pure speed optimization layered on the same core later,
  with no replay impact (it changes which equal-cost path *order* is explored only up
  to the tie-break, which still pins the result).
- **Unblocks the GTW-12 child work:** the **deterministic neighbour enumeration**
  (planar adjacency per OQ-1 + `links_from` edges, sorted) and the **weighted search
  core** (Dijkstra + tie-break + both query entry points). The exact Linear child
  ticket IDs are to be created/confirmed on the board when GTW-12 is broken down — not
  asserted here.
- **Forecloses** a BFS/uniform-step implementation and any non-keyed tie-break; undoing
  either would require a superseding ADR and would break replay.

## Alternatives considered

- **BFS / uniform-step search.** Rejected: correct only when every edge costs the
  same. With terrain 4/6/8 and a distinct flat `link_tu`, BFS returns non-cheapest
  paths. Not viable for either deliverable.
- **A\* for everything (including the overlay).** Rejected for the reachable-range
  overlay: with no single goal there is no admissible heuristic to compute, so A*
  degrades to Dijkstra anyway — better to express the overlay as the Dijkstra flood it
  natively is, and reserve the heuristic for the point-to-point query on the shared core.
- **Two separate searches (a point-to-point pather and a separate range flood).**
  Rejected: doubles the code, the cost model, and the determinism surface for no gain —
  Dijkstra subsumes both, and A* is a thin heuristic layer on it.
- **Separate ground-then-vertical pathfinding** (plan on a storey, then chain link
  entry/exit sub-paths). Rejected for now: more moving parts and coordination logic for
  no correctness benefit; a single level-aware search with links-as-edges is simpler and
  already honours §48's link pricing. Revisit only if ground vs vertical gameplay rules
  genuinely diverge.
- **Tie-break by `(cost, insertion_counter)` or by heap order.** Rejected: insertion
  order inherits `HashMap` iteration nondeterminism, and bare heap order is unstable on
  equal keys — both break byte-equal replay. The `(z, y, x)` cell key is the only
  data-only total order.
- **Flat same-cost diagonals (if 8-connected).** Rejected: lets a unit travel ~41%
  farther per TU on the diagonal, making diagonal dashes strictly optimal — a known
  exploit the genre avoids.
- **4-connected adjacency.** A genuine, lower-cost option, recorded as the live
  alternative to the tentative 8-connected recommendation. Not chosen *tentatively*
  because it under-uses the `Direction` compass and diverges from the XCOM feel — **but
  this is the user's call and 4-connected remains fully on the table.**
- **Do nothing / keep single-step movement.** Rejected: GTW-12 (multi-level path search
  - reachable-range overlay) is required for the manual-play loop; single-step `move_ganger`
  cannot preview routes or reachable footprints.

## Open questions to resolve before / at build

1. **OQ-1 ratification (BLOCKING for the neighbour-enumeration child work):** does the
   user ratify 8-connected + octile + no-corner-cutting, or choose 4-connected?
   Neighbour enumeration cannot be finalized until this is settled. The cover-facing
   count (4 vs 8) follows from it.
2. **Octile integer approximation (only if 8-connected):** orthogonal=4 / diagonal=6
   (ratio 1.5, on the existing min-4 scale) vs a tighter `≈1.41×`? A tuning leaf, but
   it must be authored before the search core's cost function is final. (The 4/6 example
   is an inference from the existing min-cost-4 scale, not yet authored canon.)
3. **`link_tu` per-kind split:** §48 currently prices *one* flat `link_tu` for every
   link kind (confirmed in `tuning/economy.rs`: `LinkTu(u8)`, "ONE flat cost for every
   link kind"). Confirm stairs and ladders stay the same flat cost (assumed yes); if
   they ever differ, the search edge cost and the heuristic's `min_link_cost` bound both
   move.
4. **Straightness/larger-`g` tie-break tier:** ship the bare `(cost, cell_key)` order
   first, or include a larger-`g` tier above the cell key for straighter-looking paths?
   Cosmetic, must not displace the final cell-key tier.
5. **Path representation at the API boundary:** does the search return a full
   `Vec<CellLevel>` route (fits the current one-message-per-step dispatch and easy
   preview) or a richer route handle with per-step cost metadata? Affects the
   pathfinder ↔ dispatch seam, not the economy.
6. **Visibility coupling (defer to GTW-13):** §48 plans routes on *true geometry* but
   bends around *visible-or-remembered* blocking scatter only. GTW-12's core can stay
   FOV-agnostic; confirm the scatter-visibility filter is layered in by the FOV ticket,
   not baked into the search.
7. **Child ticket IDs (Linear):** the GTW-12 breakdown (neighbour enumeration; search
   core) needs its child tickets created/confirmed on the board before this ADR cites
   any specific GTW-* IDs for them.
