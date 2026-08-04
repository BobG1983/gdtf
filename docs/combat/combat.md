# Battlescape (TBS Combat)

The tactical layer. **Square grid** — locked. Cover, blast radii, building footprints, destructible terrain, and the entire genre's (and the player's) muscle memory assume square tiles. There is no upside to hex combat.

Grid topology is sim-side and engine-agnostic: a cell is `glam::IVec2` and a position is the pair `(cell, level)`. The square grid is the **rule model** (in `gdtf_battle_sim`); how it's *drawn* is a separate presenter concern (below).

## Presentation — settled

The intended *eventual* look is **isometric projection** (UFO:EU-style dimetric), **fixed camera — pan + zoom only, no rotation**. Isometric is a *rendering* choice; the grid topology is unchanged (still square). Rotation is deliberately excluded to keep art at **1× per unit/prop** — a rotatable view would demand 4–8× the sprites, which fights the minimize-art-cost strategy; the original X-COM was fixed iso for exactly this reason.

**The shipped first pass is a top-down 16×16 SPRITE renderer**, not iso. `gdtf_battle_presenter` draws the battle on a dedicated world camera (`WorldCamera`, beneath the UI camera) from the role-separated `alt_tileset_terrain` / `alt_tileset_characters` / `alt_tileset_effects` sheets (one `TextureAtlasLayout` per sheet), via a single `CELL_PX` (= 16.0) cell↔world bridge (`cell_to_world(cell, level)` / `world_to_cell`) and a per-level draw-z. The sim's 8 facings collapse to the sheet's 4 sprite frames through the pure `facing_frame` map (N/NE/NW→UP, E→RIGHT, SE/S/SW→DOWN, W→LEFT). The renderer is chosen by the `BattlePresenterMode` enum; the **isometric renderer is the deferred alternate behind that enum**.

**TBD (Bevy):** the *iso* render setup — camera type (orthographic dimetric camera vs. 2D iso), the iso cell→screen projection, and any iso-specific camera controller — is pinned when the deferred iso renderer is built. The top-down camera, projection, and window/render config (design resolution, nearest-neighbor texture filtering, the app-wide UI theme) are landed in `gdtf_battle_presenter` + the `gdtf_app` window/render plugins, configured in the Bevy `App`.

## Arena size

The ceiling and reference numbers come from *UFO: Enemy Unknown* (source: UFOpaedia, Battlescape Map Generation):

| Map type | Size |
| ---------- | ------ |
| Hard maximum | **60×60×8** |
| Typical | 50×50×8 |
| Base Assault | 60×60×8 |
| Smallest (small/medium scout crash) | 40×40×4 |
| Standard maps | up to 8 elevations |
| Tunnel War maps | 2 elevations, always 60-tile areas |

**Decisions:**

- **60×60 is the hard max** — battle-tested and correct.
- **Cap the *common* mission at 40–50.** 60×60 is exactly where the last-enemy hunt becomes the infamous slog; reserve it for set-pieces (base defense, the big turf showdown). Pacing is part of whether generated situations stay fun.
- **8 Z-levels** — the full 60×60×8 battle-space. Verticality is load-bearing for situations (rooftop overwatch, roof campers) and is **not** deferred: the model is **level-true end to end**. The coarse occupancy/surface grids carry all **8 storeys** (a `MAX_LEVELS` constant on the sim's coarse-occupancy type; maps may author right up to the 8-level ceiling), position everywhere is the pair `(cell, level)`, and gangers change storeys **only over authored stair/ladder links** (a situation's `vertical_links`, validated and poured into the movement graph). The view slices **X-COM-style** — every layer above the active view level is hard-hidden (presenter sets those entities to `Visibility::Hidden`, no dimming). **TBD (Bevy):** map authoring format — situations are Bevy assets (a custom asset type loaded via the asset server / a `bevy_reflect` scene); the greybox fixtures (a multi-storey city with door gaps, roof stairs, interior floors, and a tower; a small two-storey proof — platform, stair, roofed room with a rooftop enemy) are content to re-author as gdtf assets. Taller maps are content authoring, not engineering.

## Core mechanics (v0)

- **Time Units (TU):** every action (step, turn, snap / aimed / auto shot, kneel) costs TUs from a per-turn pool; unspent TUs fund reaction fire. Economy + stat derivations in [stats.md](stats.md).
- **Firing arc:** a shooter fires directly only at a target inside its **facing arc** (a tunable cone, default ~120° = ±60°); a target outside the arc fires **only when the shooter can afford BOTH the turn-into-arc AND the shot** (it turns to face, then fires) — else the shot is **rejected** (no TU spent, no turn, no shot). Full model: [resolution.md](resolution.md) §1.
- **Cover:** a physical object with a height plus its own armor stats & HP; it stops any round not flying strictly above its height band and can be shot and destroyed. Full model: [resolution.md](resolution.md).
- **Line of sight (LOS):** determines what a ganger can see and target — probed over the same coarse geometry the shot flies through. The squad's fog-of-war built on it (Visible / Explored / Unseen, asymmetric sight, rendered-only planning): [visibility.md](visibility.md).
- **Accuracy:** a **dispersion cone** (not a to-hit %) — width from weapon spread × stability × aim mode × fire-mode × recoil; **Shooting** × weapon accuracy sets how tightly shots cluster inside it; shots are real projectiles that travel and hit the first thing in their path (incl. cover and other gangers). Full model: [resolution.md](resolution.md).
- **Damage:** applied on a hit; modified by the weapon/armor matchup (see [matchup.md](matchup.md)).
- **Death & downing:** the HP + Wounds two-track model ([stats.md](stats.md)) decides when a ganger goes out. **Death happens in battle**; downed survivors carry Wound markers that roll on the Injury table afterward (see [wounds-and-roster.md](wounds-and-roster.md)).
- **Battle outcome (win / loss):** the fight ends when one gang has no fighter left **in the fight** — a ganger is *out of the fight* when `Downed` or `Dead` (see [Glossary](../glossary.md); the downing beat above and [wounds-and-roster.md](wounds-and-roster.md) define those states). The player's gang **wins** when every enemy gang is out of the fight while a player ganger still stands; it **loses** when its whole gang is out. A **mutual wipe** (the last player and last enemy fall together) resolves to a **loss** — winning requires a surviving player. Existence ("are there enemy gangers at all?") is grounded in the **roster fielded at setup**, not a live entity scan, so a wiped-out enemy gang still counts as fielded; an empty enemy roster never wins. The sim only *signals* this (`BattleWon`/`BattleLost`); ending the battle / aftermath transition is the surrounding scene's job.

## Ganger stats

Designed — see [stats.md](stats.md): 7 direct attributes feeding computed combat stats (Time Units, Shooting, Fight, Reactions, HP, Wounds, Morale, Bottle). Stats exist to make the wound table *bite* — a wound that drops an attribute must be felt.

## Deferred

- Blast radii (square grid is chosen partly to support these later), etc. (Z-levels / verticality is **in scope** — see Arena size above.) Destructible terrain is **in scope** and built (destructible cover + floor/roof slabs — see [resolution.md](resolution.md) §3 / §3.1). **Fall damage is in scope** too: a ganger standing on a slab destroyed under it **falls** and takes damage — see [resolution.md](resolution.md) §3.1 (the *Falls* note).
- Suppression and other advanced combat effects — **TBD (design)**, sequence after the core loop is proven.

Note: reaction fire is **not** deferred — it's intrinsic to the Time Units economy (designed, not yet built). Morale/**Bottle** is designed (see [stats.md](stats.md) and [wounds-and-roster.md](wounds-and-roster.md)).

## Resolution model — settled

The full attack pipeline (dispersion accuracy, projectile travel, physical/destructible cover, probabilistic hit-location, damage, wounds, opposed-Fight melee, intrinsic reaction fire, bleed-out) is designed — see [resolution.md](resolution.md). Remaining open work is numeric **tuning** (TU costs per action, accuracy/kickback values, clearance band edges, body-part weights, severity distribution, reaction cap, bleed rate), tracked there.

## Movement — adjacency and search

**8-connected movement, octile diagonal cost, no corner-cutting.** Ratified 2026-06-22.

Gangers move on 8 directions, not 4, matching the 8-way `Direction` compass the sim
already uses for facing. XCOM: EU is the precedent — true 8-directional tile movement
with diagonals weighted heavier. Necromunda offers no grid precedent either way; it is
inch-based and gridless.

Two sub-rules are mandatory, and both are exploit fixes rather than taste:

- **Diagonal cost is octile — about √2 × orthogonal.** With integer TU the ratio is
  approximated (orthogonal 4 / diagonal 6, ratio 1.5, on the existing min-cost-4 scale).
  Flat same-cost diagonals are rejected outright: they make diagonal dashes strictly
  best and hand back ~41% free distance per step.
- **No corner-cutting between two edge-adjacent blocked cells.** A diagonal step is
  illegal when both shared-edge orthogonal neighbours are blocked — otherwise a ganger
  phases through the corner where two walls meet. This matters constantly on cover-heavy
  maps.

**Search is one weighted uniform-cost core (Dijkstra)** over the same cost function, with
a deterministic tie-break so a replay from a seed produces the same route. The search is
adjacency-agnostic — it works identically for 4- or 8-connected, so the choice above
locks nothing in the algorithm.

Routing also excludes never-seen cells; see [visibility.md](visibility.md) for the
UNSEEN rule, which is a fog constraint rather than a movement one.

**Code site:** `crates/gdtf_battle_sim/src/perception/pathfinder/`.
