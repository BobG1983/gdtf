---
name: "ADR 0005: Grid pathfinding — adjacency, search, and vertical stitching"
description: Resolve pathfinding forks — adjacency/diagonal policy (user-ratified) and the deterministic weighted-search core that serves both point-to-point routing and the reachable-range overlay over per-storey grids stitched by VerticalLinkGraph.
---

# 0005. Grid pathfinding — adjacency, search, and vertical stitching

## Status

`Accepted` — 2026-06-22 (user-ratified). Proposed 2026-06-21.

This ADR resolves the two design forks pathfinding was parked on. **OQ-2 (search
algorithm + deterministic tie-break)** is an engineering-determined call and is
**accepted** as recommended below. **OQ-1 (adjacency / diagonal / corner-cutting)**
is a tactical-feel call the user parked deliberately; the recommendation here was
**ratified by the user (2026-06-22)** — 8-connected + octile diagonal cost +
no-corner-cutting is the chosen model. The user also ratified the **movement-interaction
spec** and the **cross-storey targeting** rule (new section below), and resolved the
**stairs sprite** open question.

## Context

GDTF's core loop carries a scarred roster from fight to fight, and the fights are
turn-based tactics over a **60×60×8 cubic-voxel grid** ([battle-space.md](../combat/battle-space.md)):
x/y are ground-plane cells, z is a storey level, one sim unit = one cell = one
level, and the canonical identity is `CellLevel` wrapping `IVec3` (`(x, y, level)`)
(`crates/gdtf_battle_sim/src/foundation/metric/coords.rs`). There are no pixels in the model.

Pathfinding owes the sim two deliverables:

1. **Point-to-point routing** — given a start and a goal `CellLevel`, the cheapest
   legal route across one or more storeys.
2. **A reachable-range overlay** — every `CellLevel` reachable within a TU budget
   (the move-preview / highlight footprint).

Several layers underneath are **already built and fixed**, and this ADR does **not**
redesign them:

- **The movement economy is canon** ([visibility.md `## Pay-per-step TUs`](../combat/visibility.md),
  referred to below as §48 — the in-repo shorthand for that section, matching the
  code comment in `tuning/economy/movement.rs`). TU charges land **per step**: the committed-walk
  writer `advance_walk` charges the entered cell's terrain `move_cost`; a vertical-link hop
  charges the flat `link_tu` **instead of** terrain (`crates/gdtf_battle_sim/src/tuning/economy/movement.rs`). The
  **move-commit step gates full-route affordability once, up front**; per-step charges then
  always succeed (strictly turn-based). Interruptions are arithmetic-free — *charged = ground
  covered*; the old spend-plus-refund economy is deleted. **Pathfinding must honour
  this economy, not invent its own.**
- **Per-terrain weighted cost is fixed**: `MoveCost(u8)` per `TerrainKind`, defaults
  `open=4, cover=6, wall=8` (`tuning/economy/movement.rs`). **The minimum positive move cost
  on the grid is 4** — load-bearing for the search heuristic below.
- **The OccupancyGrid** is the authoritative blocked/walkable surface, change-driven
  (maintained in place, never rebuilt per shot)
  (`crates/gdtf_battle_sim/src/terrain/occupancy/grid/storage.rs`).
- **The VerticalLinkGraph** indexes authored stair/ladder links, validated at setup,
  bidirectional unless one-way, queryable via `links_from(origin)`
  (`crates/gdtf_battle_sim/src/terrain/vertical/graph.rs`). It is explicitly **existence-only
  today — traversal / pathfinding / movement cost is this ADR.**
- **Determinism is a hard constraint**: the sim is seeded-replayable; equal-cost
  choices must resolve in a fixed total order or two replays diverge. Movement reads
  consume no RNG. The established precedent is the `auto_select` cell-key sort by
  `(z, y, x)` — **not** entity-ID order (`cell_order_key`).

What is **not yet decided**, and why a decision is needed now:

- **OQ-1 — adjacency model**: 4-connected (orthogonal only) vs 8-connected (with
  diagonals), plus, if 8-connected, the **diagonal cost** and the **corner-cutting
  legality** policy. This shapes the neighbour-enumeration that the search graph is
  built from, and it is fundamentally about how movement *feels*.
- **OQ-2 — search algorithm + deterministic tie-break**: which search powers *both*
  deliverables, and how equal-cost frontier expansions are pinned for byte-equal
  replay.

There is no pathfinding code today; `move_ganger` steps a single `dest: CellLevel`
per call (since evolved into the committed per-tick walk,
`crates/gdtf_battle_sim/src/acts/movement/walk.rs`). This ADR introduces the
search and the neighbour model; it does not touch the economy.

## Decision

### OQ-1 — Adjacency, diagonal cost, corner-cutting *(tactical-feel — user-ratified 2026-06-22)*

This is a design/feel call, not an engineering-correctness one, and the user parked
it on purpose. We present both honestly. The user accepted the recommendation
("That's fine"), so **8-connected + octile diagonal cost + no-corner-cutting is the
chosen model**; the 4-connected option below is recorded as the rejected alternative.

**4-connected (orthogonal only)**

- For: simplest to build; clean axis-aligned cover facings (4 faces) that are easy to
  reason about; no diagonal-cost question; no corner-cutting rule needed.
- Against: blocky, staircase routes around obstacles; coarse flanking angles; under-uses
  the existing 8-way `Direction` compass (`crates/gdtf_battle_sim/src/combatants/ganger/direction.rs`,
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

With 8-connected ratified, two sub-policies are mandatory (they are exploit-fixes,
not taste):

- **Diagonal cost = octile (≈ √2 × orthogonal).** With integer TU we approximate the
  ratio (e.g. orthogonal=4 / diagonal=6, ratio 1.5, reusing the existing min-cost-4
  scale; a tighter `≈1.41×` is possible). Flat same-cost diagonals are rejected — they
  make diagonal dashes strictly best.
- **No corner-cutting between two edge-adjacent blocked cells** (the
  `AT_LEAST_ONE_WALKABLE` rule): a diagonal step is illegal if both shared-edge
  orthogonal neighbours are blocked. Matters on cover-heavy grimdark maps.

> **Decision (user-ratified 2026-06-22): adopt 8-connected with octile diagonal cost
> and the no-corner-cutting rule.** Rationale: it matches the XCOM precedent GDTF is
> chasing and reuses the load-bearing `Direction` compass. The user accepted this
> recommendation ("That's fine"). Critically, OQ-2 below is **adjacency-agnostic** — it
> works identically for 4- or 8-connected, so this choice locks nothing in the
> search/tie-break.

### OQ-2 — Search algorithm + deterministic tie-break *(engineering-determined — accepted 2026-06-22)*

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
total order `auto_select` already uses. The cell key as the **final** component gives
a data-only total order that depends on map data alone, not heap internals, insertion
timing, or `HashMap` iteration. An optional straightness/larger-`g` tier may sit *above*
the cell key for nicer-looking paths, but never *instead* of it. **Neighbour enumeration
must also be sorted deterministically** (planar neighbours and `links_from` results
iterated in a fixed order), or any intermediate tier silently reintroduces nondeterminism.

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

## Movement interaction (user-ratified 2026-06-22)

The pathfinding core (above) is the sim plumbing; this section captures the
**player-facing movement loop** the user ratified on 2026-06-22, which sits on top of
that core and is faithful to the §48 economy.

### Two-click select → preview → commit

With a ganger selected, movement is a **two-click** interaction:

1. **First click — select target + reveal path + show cost.** Clicking a target square
   selects it, **reveals the path** to it (the point-to-point route from the OQ-2 core),
   and **shows the planned move's TU cost above the target cell**. Pathing into areas the
   player **cannot see is not allowed** — the route may only run through visible cells, so
   a target requiring a path through the unseen does not preview a route.
2. **Second click — commit + walk.** Clicking again **commits** the move. Commit is the
   **single up-front affordability check** (§48): the move-commit gate verifies the full
   planned route is affordable, once, here — exactly the §48 gate the search feeds. On
   commit the ganger starts a **tweened, per-step-charged, interruptible** walk along the
   previewed route.

### Tweened, per-step, interruptible walk

- **All movement is tweened** from the start cell to the end cell — the ganger animates
  smoothly between cells; nothing snaps.
- **TU is spent per step taken, not all at once** (consistent with §48 — *charged = ground
  covered*). The committed-walk writer (`advance_walk`: terrain cost / vertical-link
  `link_tu`) charges each entered cell as the walk progresses; the commit gate guaranteed
  affordability up front, so each step's charge always succeeds.
- The walk **stops** when an **enemy is revealed**, or when a **reaction shot is fired** —
  either is an interrupt that halts the walk where the ganger stands. This is arithmetic-free
  and consistent with the §48 charged = ground-covered economy: the walk simply **stops
  paying** at the interrupt; there is no refund of the un-walked remainder, because the
  un-walked cells were never charged.

### Cross-storey targeting (OQ-4)

Clicking a **vertical-link tile** (stair / ladder) does **not** change the active level and
does **not** target the link. To move to another storey, the player **clicks a tile on that
storey** — selecting another storey switches the active-level view, and the destination tile
on that storey is the click-target. The route then **routes through the vertical link**
(per *Vertical stitching* above), but the link tile itself is **never** the target; the
destination cell on the other storey is.

### How the children carry each part

- **Path-preview** — first-click route reveal over the OQ-2 point-to-point core,
  including the "no pathing into unseen" constraint.
- **Range / cost-display** — the TU cost shown above the target cell (and the
  reachable-range overlay from the OQ-2 Dijkstra flood).
- **Dispatch-constrain + commit** — second-click commit wired to the §48 up-front
  affordability gate, then dispatching the route to the per-step move writers.
- **Tween + interrupt** — the tweened per-step walk and the stop-on-enemy-revealed /
  stop-on-reaction-shot interrupt.
- **Cross-storey input** — click-on-destination-storey targeting (OQ-4): switching
  the active-level view by storey selection and routing through the link, never targeting
  the link tile.

## Consequences

- **Both deliverables share one core.** The reachable-range overlay and
  point-to-point routing differ only by `h` (≡ 0 for the flood, `chebyshev × 4` for
  routing) — one priority-queue, one relaxation, one tie-break to test and maintain.
- **The economy contract is preserved, not duplicated.** Pathfinding reads
  terrain/`link_tu` costs and totals them; it never charges TU and never adds refunds.
  The move-commit gate remains the only affordability check (§48).
- **Replay stays byte-equal** as long as the `(cost, …, (z,y,x) cell_key)` order and
  sorted neighbour enumeration are honoured. This is a **testable invariant** — a
  determinism regression (same map + budget → identical route and identical reachable
  set) should ship with the search core.
- **Determinism note must be enforced in code**: keyed/cell-based lookups only; no
  `Entity`-ID-order iteration, no raw `HashMap`-order iteration in the hot loop.
- **With 8-connected ratified**, the build owes the octile cost *and* the
  no-corner-cutting rule together; shipping diagonals without both reopens the two
  exploits. The cover-facing model is the 8-face one (ratified).
- **A* is deferrable.** Shipping Dijkstra-only (h ≡ 0) for routing is correct, just
  slower; the heuristic is a pure speed optimization layered on the same core later,
  with no replay impact (it changes which equal-cost path *order* is explored only up
  to the tie-break, which still pins the result).
- **Unblocks the child work:** the **deterministic neighbour enumeration**
  (planar adjacency per OQ-1 + `links_from` edges, sorted) and the **weighted search
  core** (Dijkstra + tie-break + both query entry points).
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
- **4-connected adjacency.** A genuine, lower-cost option, **rejected** (user-ratified
  2026-06-22 in favour of 8-connected): it under-uses the `Direction` compass and
  diverges from the XCOM feel the project chases. Recorded here as the considered-and-
  rejected alternative; revisiting it would require a superseding ADR.
- **Do nothing / keep single-step movement.** Rejected: multi-level path search and the
  reachable-range overlay are required for the manual-play loop; single-step `move_ganger`
  cannot preview routes or reachable footprints.

## Open questions

### Resolved (user-ratified 2026-06-22)

- **OQ-1 — adjacency / diagonal / corner-cutting: RESOLVED.** The user ratified
  **8-connected + octile diagonal cost + no-corner-cutting** ("That's fine" to the
  recommendation above). Neighbour enumeration is 8-connected; the cover-facing count is
  8 faces. (The octile *integer* approximation is a separate tuning value — still open
  below.)
- **OQ-3 — stairs vertical-link sprite: RESOLVED.** The stairs tile is **atlas index 77**
  (authored as **row 5, column 14**, 1-indexed → 0-based `(5-1) × 16 + (14-1) = 77` on the
  16-wide atlas). The **ladder stays atlas 235**.
- **OQ-4 — cross-storey targeting: RESOLVED.** Clicking a vertical-link tile does **not**
  change the active level or target the link; the player clicks a tile **on the destination
  storey** (which switches the active-level view) and the path routes through the link. The
  link tile is never the target. Captured in *Movement interaction* above.
- **OQ-5 — movement interaction: RESOLVED.** Tweened movement, two-click
  select/preview/commit, per-step TU charge, TU cost shown above the target cell,
  stop-on-enemy-revealed / stop-on-reaction-shot interrupt, and no pathing into unseen
  areas. Captured in *Movement interaction* above.

### Still open — to resolve before / at build

1. **Octile integer approximation:** orthogonal=4 / diagonal=6 (ratio 1.5, on the existing
   min-4 scale) vs a tighter `≈1.41×`? A tuning leaf, but it must be authored before the
   search core's cost function is final. (The 4/6 example is an inference from the existing
   min-cost-4 scale, not yet authored canon.)
2. **`link_tu` per-kind split:** §48 currently prices *one* flat `link_tu` for every link
   kind (confirmed in `tuning/economy.rs`: `LinkTu(u8)`, "ONE flat cost for every link
   kind"). Confirm stairs and ladders stay the same flat cost (assumed yes); if they ever
   differ, the search edge cost and the heuristic's `min_link_cost` bound both move.
3. **Straightness/larger-`g` tie-break tier:** ship the bare `(cost, cell_key)` order
   first, or include a larger-`g` tier above the cell key for straighter-looking paths?
   Cosmetic, must not displace the final cell-key tier.
4. **Path representation at the API boundary:** does the search return a full
   `Vec<CellLevel>` route (fits the current one-message-per-step dispatch and easy
   preview) or a richer route handle with per-step cost metadata? Affects the
   pathfinder ↔ dispatch interface, not the economy.
5. **Visibility coupling:** §48 plans routes on *true geometry* but bends around
   *visible-or-remembered* blocking scatter only. The search core can stay FOV-agnostic;
   the scatter-visibility filter is layered in by FOV work, not baked into the search.
   (This is also the source of the "no pathing into unseen" filter in *Movement interaction*
   above.)
