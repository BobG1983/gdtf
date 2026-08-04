# Visibility — squad fog-of-war

What the player's side can see, remember, and act on. **Three states per (cell, level)**, computed model-side (`gdtf_battle_sim`) from the **squad-combined point of view** — one fog for the player side, the union of every ganger's eyes, not per-ganger overlays. The presenter presents it; it never owns it (the model/view split is in [ADR 0001](../decisions/0001-rust-bevy-rewrite.md)).

## The three states

| State | Meaning | Presents as |
|-------|---------|-------------|
| **VISIBLE** | Some conscious squad ganger currently sees it | Normal |
| **EXPLORED** | Seen at some point this mission, not currently | Full brightness, **greyscale** (BT.709 luma — colour removed, not dimmed) |
| **UNSEEN** | Never seen this mission | Hidden — the dark clear color reads through |

All three are **per (cell, level)** (`IVec3` keys on the model's sets): a fogged upper storey can sit over a visible ground floor. Reads are pure set lookups (`is_cell_visible` / `is_cell_explored` / `is_ganger_visible`); the one writer is `recompute_visibility`, run on every trigger that can change what the squad sees (moves, life flips, real stance changes, geometry destruction, setup's spawn FOV).

## Per-ganger FOV, the squad union, the mission memory

A ganger F sees (cell, level) iff **`Chebyshev(F.cell, cell) <= view_range` AND `has_los(F.cell, F.level, cell, level)`** — the same level-aware coarse-geometry probe the shot pipeline flies through ([resolution.md](resolution.md); ONE geometry truth, never a second LOS). Only **conscious** (`is_active` — ALIVE) **player-faction** gangers observe: Downed/Dead gangers see nothing, enemies feed no squad FOV. The squad's **VISIBLE** set is the **union** of every observer's FOV; **EXPLORED** accrues from it and is **monotone per mission** — it only ever grows, surviving every recompute (`recompute_visibility`).

Memory shows **live terrain in greyscale, not snapshots**: the fog treatment modulates the *current* rendered geometry, and destruction erases tiles / frees props at impact time regardless of fog — nothing ever repaints from a stored snapshot. An EXPLORED tile renders at its **full live brightness with its colour removed** (the perceptual BT.709 luminance) — colour-loss, not brightness-loss, is the memory cue (user decision 2026-06-21). A wall destroyed while fogged vanishes from the greyscale memory too; the player's "map memory" is honest about geometry without leaking who destroyed it.

## Asymmetric sight (user ruling 2026-06-12 — "No forced symmetry is excellent")

The LOS probe's anchors are **directional**: it launches at the **shooter's eye/muzzle height for its stance** (ganger data `shot_z_by_stance` via `_los_start`) and lands on the **target's derived aim point** (`target_aim_point` via `_los_aim` — stance silhouette × `aim_height_frac`, a corpse aimed low, cover at its band midpoint). Swap the two gangers and the probe flies a *different ray*, so sight across height-banded cover can be **one-directional** — a low watcher may see a tall target that cannot see it back. This is by design, not a defect: **the unseen shooting the seen is intended gameplay** (the "muzzle flash in the dark" read — enemy FX deliberately aren't fog-gated). **TBD (Bevy):** enemy muzzle-flash FX bypassing the fog gate is a presenter rule, pinned when FX staging lands.

**AI symmetry**: enemies consult the **same `can_see`** (the identical Chebyshev × `has_los` composition, restated per observer/target pair) about their *own* eyes as the engagement gate. The squad sets are the *player's* fog; the AI is never gated on what the player sees.

## Composition with the view slice

Two visibility writers, two flags, never crossed (the same separation as [battle-space.md](battle-space.md) "the view slice"):

- the **slice owns LAYER visibility** — every per-level layer **strictly above** the active view level is hard-hidden. The active level and every storey **below** it are DRAWN (the multi-level display below), so the slice's terrain band is `[0..=active]`, culling only above — not a single-storey hard cut. (That is the default `DownToActive` shape: since the general which-storeys-draw rule is the ONE shared storey-treatment classifier — `Hidden | Active | ContextBelow(depth)` per storey, with the full-stack `FullView` and the band-floor-raising `Isolate` toggle as its other modes; see [battle-space.md](battle-space.md) "The view slice".);
- **fog owns per-cell presentation and actor flags** — per-cell fog modulate on the rendered terrain, plus each Ganger/ScatterProp **entity's own** visibility/modulate (an enemy hard-cuts: no fade, no last-known ghost; a corpse shows iff its cell is squad-visible).

**Confirmed in Bevy:** the source composed these for free via the scene-tree parent-chain AND (`is_visible_in_tree`); Bevy's presenter has no layer-parent hierarchy to inherit through, so it composes the two facts explicitly, per surface. For **terrain**, the slice decides which tiles are drawn (the `[0..=active]` band) and the fog writer (`present_fog`) modulates every drawn tile in place. For **actor sprites**, ONE pure classifier ANDs the band fact and the fog fact, and ONE resolver (`resolve_ganger_visibility`) writes each sprite's visibility — a thing draws iff fog shows its entity/cell AND the slice shows its storey, decided in exactly one place; when the fog sets are not resident (a focused harness) the classifier's absent-fog branch is band-only. Both writers are change-tick-quiet (visibility via `set_if_neq`, material knobs compared before any tracked mutate), so an unchanged fog state re-dirties no component and re-uploads no material uniform. The principle holds: the **rendered layer IS the fog mask** — only authored, still-standing terrain has a cell to modulate, so fog can never present over void (the model's visible/explored keys extend over open air by design) and destroyed terrain stays destroyed. The model keeps the visibility wire and a public `present_fog` function for the presenter.

### Multi-level display: the storey-DEPTH darken (a separate, composing axis)

The presenter draws terrain **bottom-up from storey 0 up to the active view level** (the UFO:EU / OpenXcom multi-level display, user ruling 2026-06-23) — everything strictly above `active` is culled, and **open/empty upper cells emit nothing** so a floor-gap on storey *k* reveals the storey *k-1* cell beneath (peek-through: a sprite is emitted only where real terrain — a wall / cover / slab — exists on that storey; storey 0 keeps its full floor field). Per-storey Z gives the painter's-algorithm occlusion for free (a higher storey draws in front).

Non-active drawn storeys are **DARKENED**; the active storey is **full-bright**. This **storey-depth darken is a brightness-loss axis, and it is DISTINCT from the fog EXPLORED greyscale (a colour-loss axis)** — the two are orthogonal treatments that **COMPOSE, never replace one another**:

- **fog EXPLORED = colour-loss, NOT brightness-loss** (the memory cue, unchanged canon above and in [Tunables](#tunables-combat-tuning-authored-as-a-loaded-asset)): a remembered cell renders at *full brightness*, greyscale — this is deliberately **not dimmed**;
- **storey depth = brightness-loss, NOT colour-loss**: a lower *drawn* storey renders *dimmer* than the active storey so the player reads which floor is "underneath", regardless of that storey's fog state.

So a **lower-storey EXPLORED tile ends up BOTH greyscaled (fog) AND dimmed (depth)** — the shader mixes toward BT.709 grey by `(1 - saturation)`, **then** scales the result by a `brightness` multiplier (`1.0` active / `< 1.0` lower drawn storey). The presenter drives both knobs on the one `TerrainFogMaterial` (`saturation` for fog, `brightness` for depth). This surfaces — rather than silently resolves — the apparent tension with the "EXPLORED is not dimmed" rule: EXPLORED itself still never dims; the *depth* dim is a second, independent reason a tile may be darker, applied by storey, not by memory. The darken magnitude is a single flat tunable (no per-depth ramp yet — a later in-engine-discovery tune). Lower-storey **units** and a full-view **toggle** are separate follow-ons; this axis covers terrain only.

## Rendered-only planning

The player **plans around exactly what is rendered**, and nothing more:

**Routing excludes UNSEEN** (user-ratified OQ-5, 2026-06-22; reverses the old "plan through the dark", see [ADR 0005](../decisions/0005-pathfinding-adjacency-and-search.md)): the path search **must not route INTO cells the squad cannot currently SEE**. An UNSEEN `(cell, level)` — in *neither* the VISIBLE nor the EXPLORED set — is **non-routable** (impassable for planning), so the route bends around it within the non-UNSEEN set and a destination reachable only by crossing UNSEEN is no route at all. **EXPLORED (remembered) cells remain routable** — the XCOM model: you plan through what you remember, just not the true dark. So **ROUTABLE = non-UNSEEN = VISIBLE ∪ EXPLORED**.

Within routable cells the blocking predicate is visibility-aware:

- **Own squad** always blocks a route (player ids are trivially visible); an **enemy blocks iff squad-VISIBLE** — the same `is_ganger_visible` read that shows/hides its entity, so plan and render can never disagree (the roster query for other-ganger positions).
- **Blocking scatter** joins the planning solids on **non-UNSEEN** cells only (the pathfinder's scatter-planning-blockers) — a prop the player can see or remembers bends the route; an unseen one never does (and its cell is non-routable anyway, so the non-UNSEEN bound holds by construction).
- **Walls/floor stay geometry truth** — the route plans **on true geometry over the routable (non-UNSEEN) cells**; UNSEEN is non-routable. Fog hides *occupants* and the *never-seen dark*, not the bones of what you can see or remember.

What the fog *currently* hides never bends a preview into it — routing around an unseen body would leak its position, and routing INTO the never-seen dark is forbidden outright. **The ambush** is the enforcement at walk time and is **kept** (user move-interaction ruling): before *entering* each next cell the committed walk checks live model truth (a living ganger there, or blocking scatter) and **bump-stops at the last free step** — no teleport, no co-location. Each accepted step's writer recomputes the squad FOV, so the ambusher is **revealed iff the mover's own sight reaches it from where it stopped** — the per-step recompute is the reveal mechanic; there is no extra one.

## Pay-per-step TUs

Movement TU **charges land with each step**: the committed-walk writer `advance_walk` carries the charge atomically with the position write — it charges the entered cell's terrain `move_cost` on a planar step, and the flat `link_tu` at a link hop (a crossing prices `link_tu` *instead of* terrain). The **commit gates full-route affordability once**, up front (the move-commit step); the per-step charges then always succeed (strictly turn-based — nothing else can spend the mover's TU mid-walk). **Interruptions are arithmetic-free**: a bump-stopped or race-stopped walk simply stops paying — **charged = ground covered**, by construction; the old up-front-spend-plus-refund economy is deleted. An uninterrupted walk sums bit-identical to the preview's price (to be pinned by a `tu_per_step` test).

## UX edges

- **Walking into the never-seen dark is refused** (user-ratified OQ-5, 2026-06-22; reverses the old "walking into the dark is allowed", see [ADR 0005](../decisions/0005-pathfinding-adjacency-and-search.md)) — the route preview and the commit **exclude UNSEEN cells**: you cannot path into cells you have never seen. **Walking into EXPLORED (remembered) territory stays allowed** — an EXPLORED walkable destination previews and commits at the normal geometry price (no fog premium); only the true, never-seen dark is off-limits to routing.
- **EXPLORED path steps stay routable** — a previewed route may run over remembered (EXPLORED) cells at the normal price; it never crosses an UNSEEN cell (those are non-routable, so no "unknown" step exists on a preview to highlight). **TBD (Bevy):** any memory-tint on the previewed route's remembered steps is a presenter concern.
- **The cursor leaks no walkability on a non-VISIBLE cell** — the box reticle never betrays where fog hides standing surface or a hidden body, because what lights it is gated by *occupancy*, not visibility. On a non-VISIBLE cell (UNSEEN *or* merely EXPLORED) the reticle shows for **blocking GEOMETRY only** — a wall / cover the player can remember is map memory, so it still lights, but **grey-recoloured + fire-refused** (the "unseen — hold your fire" recolour, /; see the next bullet). A **fog-hidden OCCUPANT lights nothing and is not inspectable** — a hidden enemy emits no reticle and never populates the inspect panel, closing the info-leak where a grey reticle (or a stat block) betrayed where the fog hid a body. An **empty unseen cell lights nothing** — bare floor never gets a reticle regardless of visibility, so the cursor reveals no walkability there. EXPLORED *terrain* is mission memory and reads as such; it just never leaks an *occupant*.
- **Targeting is refused into any non-VISIBLE cell** (UNSEEN *or* merely explored): the reticle recolours and the fire commit refuses identically (zero TU, zero rounds, zero model mutation, targeting stays armed). There is **no text hint** — the affordance is the reticle recolour + the fire-refusal (user-ruled 2026-06-22,: the "unseen — hold your fire" hint banner floated over the HUD and was removed; the *don't-fire-into-the-unseen* rule stays). The reticle and the fire-refusal share ONE read (the presenter's `cell_squad_visible`, returning a `CellVisibility` verdict, fail-closed on an absent squad fog) so they can never disagree. The fog gate is **player policy** — it never enters the shared `can_fire` act (which stays LOS/fog-free, see [resolution.md](resolution.md) §"What's pure math vs sim").

## Cross-level tactical badges

**Every fog-VISIBLE actor in the drawn band is on screen — as its sprite or as a marker.** The multi-level display (above) hard-cuts an actor's own sprite to the drawn band (`storey <= active` by default): a squad-VISIBLE enemy on a storey OUTSIDE that band would otherwise vanish from the screen entirely even though the squad fog says it is seen right now — "Signals, Not Scenery" closes that gap. The cross-level tactical badges (`gdtf_battle_presenter::overlays::cross_level_signals`) render a compact corner badge on the ACTIVE storey standing in for what the terrain draw's hard cut hides:

- **Threat** (badge labels `ThreatAbove` / `ThreatBelow`) — a squad-VISIBLE enemy above or below the active storey. Gated by the SAME `is_ganger_visible` read that shows/hides the enemy's own sprite: an UNSEEN or merely-EXPLORED (not currently VISIBLE) enemy **never** leaks a Threat badge — plan and render can never disagree.
- **DropDepth** — a hole/ledge cell's fall distance (the same `resolve_drop` a real fall resolves), fog-gated on squad-EXPLORED (the terrain-draw treatment: an EXPLORED terrain fact stays legible, an UNSEEN one does not).
- **ConnectorDelta** — a stair/ladder endpoint's signed level-delta to its other end, the same EXPLORED gate.

Badges never invent a new fact — each is a cross-level-legible presentation of a fact the sim/terrain already computes. Per-cell aggregation dedupes Threat badges by distinct level-delta (multiple enemies at the same delta collapse into one badge with a count), and a per-cell cap keeps at most 3 badges (Threat nearest-first, then DropDepth, then ConnectorDelta), silently dropping the rest — full detail for an overflowing cell is the cell-hover HUD echo, not the badge itself.

## Tunables (combat tuning, authored as a loaded asset)

| Tunable | Default | Meaning |
|---------|---------|---------|
| `view_range` | 14 | One ganger's sight radius, in Chebyshev cells — the 2D disc bounds the range, the LOS probe owns the level axis. The shipped 60×60 city gets a real fog horizon; the 12×12 fixtures read fully lit around the squad. |
| `explored_dim` | 0.55 | **DEPRECATED / UNUSED as of.** Historically the RGB modulate factor on EXPLORED terrain (dimmer, same colour). The EXPLORED treatment is now **full-brightness greyscale** (colour-loss, not brightness-loss — user decision 2026-06-21), implemented presenter-side via the `TerrainFogMaterial` `saturation` knob (`1.0` VISIBLE colour / `0.0` EXPLORED greyscale), so it reads no tunable magnitude. The leaf is retained as a parseable, unread tuning field (was presenter-only); a future ticket repurposes or retires it. |

See [combat.md](combat.md) for where LOS sits among the core mechanics, and [resolution.md](resolution.md) for the shot pipeline the probe shares its geometry with.
