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

- the **slice owns LAYER visibility** — every per-level layer above the active view level is hard-hidden;
- **fog owns per-cell presentation and actor flags** — per-cell fog modulate on the rendered terrain, plus each Ganger/ScatterProp **entity's own** visibility/modulate (an enemy hard-cuts: no fade, no last-known ghost; a corpse shows iff its cell is squad-visible).

**TBD (Bevy):** the source composed these for free via the scene-tree parent-chain AND (`is_visible_in_tree`). In Bevy the equivalent is inherited `Visibility` over the entity hierarchy: a thing draws iff fog shows its entity/cell AND the slice shows its storey — confirmed when the presenter implements the two writers. The principle holds: the **rendered layer IS the fog mask** — only authored, still-standing terrain has a cell to modulate, so fog can never present over void (the model's visible/explored keys extend over open air by design) and destroyed terrain stays destroyed. The model keeps the visibility wire and a public `present_fog` seam for the presenter.

## Rendered-only planning

The player **plans around exactly what is rendered**, and nothing more:

- **Own squad** always blocks a route (player ids are trivially visible); an **enemy blocks iff squad-VISIBLE** — the same `is_ganger_visible` read that shows/hides its entity, so plan and render can never disagree (the roster query for other-ganger positions).
- **Blocking scatter** joins the planning solids per query, on **non-UNSEEN** cells only (the pathfinder's scatter-planning-blockers) — a prop the player can see or remembers bends the route; an unseen one never does.
- **Walls/floor stay geometry truth** — the route plans through the dark on true geometry (the UFO precedent). Fog hides *occupants*, not the map's bones.

What the fog hides never bends a preview — bending around an unseen body would leak its position. **The ambush** is the enforcement at walk time: before *entering* each next cell the committed walk checks live model truth (a living ganger there, or blocking scatter) and **bump-stops at the last free step** — no teleport, no co-location. Each accepted step's writer recomputes the squad FOV, so the ambusher is **revealed iff the mover's own sight reaches it from where it stopped** — the per-step recompute is the reveal mechanic; there is no extra one.

## Pay-per-step TUs

Movement TU **charges land with each step**: the movement writers carry the charge atomically with the position write — `sync_ganger_cell` charges the entered cell's terrain `move_cost`, `move_ganger` charges the flat `link_tu` at a link hop (a crossing prices `link_tu` *instead of* terrain). The **commit gates full-route affordability once**, up front (the move-commit step); the per-step charges then always succeed (strictly turn-based — nothing else can spend the mover's TU mid-walk). **Interruptions are arithmetic-free**: a bump-stopped or race-stopped walk simply stops paying — **charged = ground covered**, by construction; the old up-front-spend-plus-refund economy is deleted. An uninterrupted walk sums bit-identical to the preview's price (to be pinned by a `tu_per_step` test).

## UX edges

- **Walking into the dark is allowed** — movement carries no fog gate; an EXPLORED or UNSEEN walkable destination previews and commits at the normal geometry price (no fog premium).
- **UNSEEN path steps draw the unknown treatment** — a reduced-alpha highlight on the previewed route's dark steps. **TBD (Bevy):** the highlight rendering is a presenter concern.
- **The cursor leaks no walkability on UNSEEN cells** — the box reticle reads one constant for every unseen cell (it always shows) instead of betraying where fog hides standing surface; EXPLORED cells keep the real verdict — that terrain is mission memory.
- **Targeting is refused into any non-VISIBLE cell** (UNSEEN *or* merely explored): the reticle recolours and the status reads **"unseen — hold your fire"**, and the fire commit refuses identically (zero TU, zero rounds, zero model mutation, targeting stays armed). Hint and act share one read (`_cell_squad_visible`) so they can never disagree.

## Tunables (combat tuning, authored as a loaded asset)

| Tunable | Default | Meaning |
|---------|---------|---------|
| `view_range` | 14 | One ganger's sight radius, in Chebyshev cells — the 2D disc bounds the range, the LOS probe owns the level axis. The shipped 60×60 city gets a real fog horizon; the 12×12 fixtures read fully lit around the squad. |
| `explored_dim` | 0.55 | **DEPRECATED / UNUSED as of GTW-348.** Historically the RGB modulate factor on EXPLORED terrain (dimmer, same colour). The EXPLORED treatment is now **full-brightness greyscale** (colour-loss, not brightness-loss — user decision 2026-06-21), implemented presenter-side via the `TerrainFogMaterial` `saturation` knob (`1.0` VISIBLE colour / `0.0` EXPLORED greyscale), so it reads no tunable magnitude. The leaf is retained as a parseable, unread tuning field (GTW-348 was presenter-only); a future ticket repurposes or retires it. |

See [combat.md](combat.md) for where LOS sits among the core mechanics, and [resolution.md](resolution.md) for the shot pipeline the probe shares its geometry with.
