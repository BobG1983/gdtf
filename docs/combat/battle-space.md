# Battle space — the cubic-voxel sim metric

The coordinate system the shot pipeline ([resolution.md](resolution.md)) flies in. **One continuous cubic-voxel metric, no pixels anywhere in the model**: x/y are ground-plane cells, z is height in *levels*, and **one sim unit is one cell on x = one cell on y = one level on z** — cubic voxels over the 60×60×8 grid. A cell is `floor(pos.x)` / `floor(pos.y)`; a z-level is `floor(pos.z)`. The view's pixel projection never enters the model — gameplay positions are sim-unit voxel coordinates, the pixel projection is the presenter's job (`gdtf_battle_presenter`).

Positions are `glam` vectors in this metric: a model point is a `Vec3` in **sim units** (cells on x/y, levels on z — never pixels), a cell is an `IVec2`, a (cell, level) key is an `IVec3`. **TBD (Bevy):** exact component/resource names for the position and cell types are pinned when the sim crate lands them.

## Why cubic voxels

The pipeline's core object is **ONE 3D unit shot vector**, and a unit vector is meaningless across mixed units. The pre-unification design mixed cell-units on x/y with screen px on z (silhouette tops, `z_top`, muzzle heights), so no honest 3D angle existed. Making the voxel **cubic** — one unit on every axis, with **no pixel in sight** — makes the cone's angles real: muzzle origin, cone sample, DDA march, and z-band classification all speak the *same* unit (a cell-width = a level-height = 1.0), and trig works. The dispersion cone and the ray march need ONE consistent metric; cubic voxels give it with zero pixels. Anisotropic on-screen scale (a cell wider than a level is tall) is a *projection* choice and lives entirely in the presenter — the sim never sees it.

## The grid

- **60×60 cells on the ground plane** — `x` and `y`, each one cell = 1.0 sim unit.
- **8 levels on z** — `z`, each one storey = 1.0 sim unit (the `MAX_LEVELS` ceiling of the coarse grid; 8 levels span `z ∈ [0, 8)`). Position everywhere is the pair `(cell, level)`; the (cell, level) key is an `IVec3`.

These are **sim facts**, not tuning: the 60×60×8 grid, the 8 levels, and the cell / (cell, level) integer keys define the coordinate system. The `cell_center` / `pos_to_cell` helpers are pure functions; `pos_to_cell` floors — never rounds — so negative coordinates bucket correctly.

## Datums on the z axis — level-fractions, not pixels

Everything vertical is authored as a **level-fraction** (a dimensionless fraction of one level's height, `z ∈ [0,1)` within a storey) or as a **height band** — never as a pixel. A datum at level-fraction `f` on level `k` sits at `z = k + f` sim units.

| Datum | Expressed as | Home |
| ------- | -------------- | ------ |
| Ganger silhouette tops (prone / kneel / stand) | a **band** — prone Low · kneel Mid · stand High — or its band-top level-fraction | tuning (universal per-stance level-fractions — there is no per-ganger size model) |
| Cover heights (per-prop) | a **band** — Low / Mid / High | object data (cover is already band-based — see §"Cover is a physical object" in [resolution.md](resolution.md)) |
| Muzzle height by stance (prone / kneel / stand) | a tunable **level-fraction** per stance | tuning (universal per-stance `muzzle_heights` — there is no per-ganger size model) |
| Clearance band edges (LOW→MID, MID→HIGH) | tunable **level-fractions** (defaults ≈ ⅓ and ⅔ of a level) | tuning `projectile_band_edges` |
| Aim height | the target's silhouette-top **level-fraction** (a cover cell → its band midpoint) | tuning |

### Banding — the silhouette / cover-height abstraction

Banding is kept: **`HeightBand { Low, Mid, High }`**. It is the **silhouette / cover-height abstraction**, not the projectile — it pairs with stance (prone / kneel / stand) and with the stability/brace model (the brace gate suits stance to a cover *band*; see [resolution.md](resolution.md) §1). Within one level (height = 1.0 sim unit) the band edges are **level-fractions** — dimensionless and tunable: `low_mid ≈ ⅓` and `mid_high ≈ ⅔` of a level by default. These replace the old absolute px edges. Because the edges are fractions of a storey, they re-scale automatically with the voxel and carry no pixel.

Two deliberate quantities on the same axis:

- **Cover band ≠ ganger band.** A Low crate is *shorter* than a prone ganger's silhouette — designed to be shot over. They are distinct height quantities even when both fall in the same band; collapsing them into one band→height table would make the crate unclearable, so they stay separate.
- **Muzzle level-fractions are authored to land in design-sensible clearance bands**: prone fires low (can't clear even Low cover — intended), kneeling fires Mid, standing fires High, so each stance's flat fire leaves in the band the design wants.

The march itself compares the round's **continuous `z`** (within the crossed cell's level) against the occupant's band-**top** height — **strictly higher sails over; equal-or-lower impacts** (see [resolution.md](resolution.md) §2). The band abstraction supplies that band-top height; the continuous level-fraction datums feed the *aim* derivation and the authored band assignments. Any measured per-part z geometry is parked, unreferenced data — retained for a possible future per-part-geometry pass, not part of the live model.

## The shot is one 3D Vec3, ray-marched by voxel DDA

The shot is **ONE 3D `Vec3`** — a continuous origin (a sim-unit position) plus a **unit-`Vec3` direction** — ray-marched through the grid as a **true 3-axis voxel DDA** (Amanatides–Woo style, `march_vector`) in sim units. It is **not** a pixel-stepped ray: the march walks the grid cell-by-cell / level-by-level in the cubic metric. "Walk the grid as a ray-march `Vec3`." "Up" is `+z` (`Vec3::Z` in the sim basis; distinct from any screen-space orientation the presenter uses).

**Clearance** is pure band-vs-height with no pixel thresholds: at each occupied cell the round crosses, its continuous `z` (in sim units, taken within that level) is compared to the occupant / cover band-top height — **strictly higher sails over; equal-or-lower impacts**. No special cases except that **the shooter's own cell never blocks its own shot**. This reproduces the old aim-occlusion exemptions for flat fire by height alone: a standing shooter's High round clears the Low crate it braces on and a prone ally by height, not by exemption (a *prone* shooter genuinely cannot clear even Low cover — intended).

## Stance / cover / muzzle / aim heights

Every vertical datum the shot pipeline reads is a **band** or a **level-fraction**, never a pixel:

- **Occupant silhouette top by stance** → a band (prone Low / kneel Mid / stand High), or its band-top level-fraction when a continuous height is needed.
- **Cover height** → a band (Low / Mid / High) — cover is already band-based.
- **Muzzle height by stance** → a tunable **level-fraction** (prone / kneel / stand each author their own).
- **Aim point** → the target cell-center `(x, y)` plus the target's **silhouette-top level-fraction**; a cover-occupied cell is aimed at the object's **band midpoint**, so deliberately shooting a low crate works at range.

## Sub-cell precision on the ground plane

The muzzle is a real 3D point inside the shooter's cell: cell center + a per-facing forward offset, at the per-stance muzzle level-fraction on z. Any sub-cell forward offset, if kept, is expressed in **cell-fractions** (a fraction of a cell along the facing's forward vector; clamped so it can never leave the cell) — never a pixel. A barrel length "in pixels" is irrelevant and gone. This is sim space — decoupled from any view gun-sprite tip, which is the *presenter's* concern.

## Mapping to the view

The presenter (`gdtf_battle_presenter`) owns the **entire** sim→world projection. The model emits sim-unit voxel coordinates (`Vec3` / `IVec2` / `IVec3`); the presenter turns those into world transforms, and it is the **only** place a pixel exists. **The landed first presenter is the top-down 16×16 SPRITE renderer** (GTW-48); the anisotropic-per-axis / per-storey-lift / X-COM-hard-cut pieces below describe the **deferred iso renderer** (GTW-49 / GTW-10), not the shipped top-down view. Every pixel constant below is a presenter constant, not a sim / tuning value:

- **A single cell↔world bridge (landed, top-down)** — the only place that maps a cell + storey to a world position and back (`cell_to_world(cell, level)` / `world_to_cell`); gameplay code thinks in `IVec2` cells plus a storey. The landed top-down renderer uses ONE **`CELL_PX` (= 16.0)** square cell size (a cell is 16×16 world units, isotropic) plus a per-level draw-z, and the bridge is a `const` projection in the presenter. *Deferred (iso, GTW-49 / GTW-10):* the iso renderer may instead scale **anisotropically per axis** — a cell one width and a level a different height — which is pure presentation (the sim never sees a pixel, so the on-screen aspect of a voxel is the presenter's free choice); its **`cell_pitch_px`** (ground-plane cell width) and **`z_level_height`** (storey height in px) would be presenter constants, not sim or tuning data. **TBD (Bevy, iso):** the iso bridge form — a `Resource` holding the grid basis vs. a system reading a grid component — is pinned when the iso renderer is built.
- **Per-level seating (the storey lift) — deferred (iso)** — in the iso renderer a storey's **lift = `level × z_level_height`** px straight up, read off the presenter's own grid basis, so the lift is **locked 1:1 to the projection**: a model point on storey `k`'s floor (`z = k` sim units) maps exactly onto the world position of `(cell, k)` — trajectories and floors can never drift (to be pinned by a presenter test). Each upper storey's geometry would be offset by this same lift so the model march and the rendered floors land on one plane by construction. (The landed top-down view instead seats every storey on the same plane and selects the visible storey via the presenter-owned `ActiveLevel` filter.) **TBD (Bevy, iso):** whether storeys are distinct entity hierarchies offset by a transform, or a shader/layer scheme, is the iso renderer's call.
- **The view slice (X-COM hard cut) — deferred (iso)** — in the iso renderer every per-level layer above the active view level is **hidden entirely** (no dimming, no transparency); everything at-or-below renders fully. The roof consequence is intentional: at ground view a roofed building's lid slabs peel away, and anything parented on a hidden level (gangers, props, FX) hides with its layer — raise the view to spot the roof camper. (Same writer separation as [visibility.md](visibility.md).) The landed top-down view's `ActiveLevel` filter is the first cut of this single-storey selection. **TBD (Bevy, iso):** likely Bevy `Visibility` toggled per-storey root entity plus parent-chain inherited visibility — confirmed when the iso slice lands.
- **Sim→world projection of a trajectory — deferred (iso)** — projecting a sim-unit 3D point (e.g. a shot-outcome trajectory) to world in the iso renderer: x/y map through the grid's (possibly anisotropic) affine basis; **z maps 1 sim level : `z_level_height` px upward** (which is exactly why the storey lift equals `z_level_height`). Known potential cosmetic seam: the muzzle level-fractions were authored for the clearance bands, so a projected launch point can sit above any gun-sprite tip — a presentation pass, not a model issue. (The landed top-down presenter draws no shot trajectory yet; the targeting reticle / shot-arc are GTW-11.)

See the model/view split this metric sits inside in [ADR 0001](../decisions/0001-rust-bevy-rewrite.md).
