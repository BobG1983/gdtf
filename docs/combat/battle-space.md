# Battle space — the unified px metric

The coordinate system the shot pipeline ([resolution.md](resolution.md)) flies in. **One metric, px on all three axes**: x/y are ground-plane px, z is height px — the same axis. A cell is `floor(pos_px / cell_pitch_px)`; a z-level is `floor(z_px / z_level_height)`. The view's projection never enters the model — gameplay positions are battle-space px, projection is the presenter's job (`gdtf_battle_presenter`).

Positions are `glam` vectors in this metric: a model point is a `Vec3` of battle-space px, a cell is an `IVec2`, a (cell, level) key is an `IVec3`. **TBD (Bevy):** exact component/resource names for the position and cell types are pinned when the sim crate lands them.

## Why one metric

The pipeline's core object is **ONE 3D unit shot vector**, and a unit vector is meaningless across mixed units. The pre-unification design mixed cell-units on x/y with screen px on z (silhouette tops, `z_top`, muzzle heights), so no honest 3D angle existed. Fixing one px metric makes the cone's angles real: muzzle origin, cone sample, DDA march, and z-band classification all speak the same unit, and trig works.

## The two constants (combat tuning)

- **`cell_pitch_px` = 180** — one grid cell on x/y. Derived from the source tile *width* (tile_size 180×104), so the pre-existing z-px datums keep their proportions on the shared axis (a standing ganger, 175 px, ≈ one cell pitch).
- **`z_level_height` = 170** — one discrete storey of the 60×60×8 coarse grid (8 levels span 1360 px). Retuned from 200 (user ruling 2026-06-11): 170 seats a level's floor flush on the painted wall tops at game scale. It sits deliberately *below* the 175 px standing model silhouette — the overshoot is accepted; model occupants band within their own storey by construction, so nothing crosses a boundary.

Both are tuning data, threaded as parameters (the `cell_center_px` / `px_to_cell` helpers are pure functions; `px_to_cell` floors — never rounds — so negative px bucket correctly).

## Datums on the z axis

Everything vertical is authored in the same px:

| Datum | Values | Home |
| ------- | -------- | ------ |
| Ganger silhouette tops (prone / kneel / stand) | 55 / 125 / 175 | coarse-occupancy height, from ganger data `height_by_stance` |
| Cover heights (continuous, per-prop) | crate 35 · barrel 85 · stack/wall 120 | object data `z_top` (authored per object) |
| Muzzle height by stance (prone / kneel / stand) | 35 / 100 / 150 | ganger data `shot_z_by_stance` |
| Clearance band edges (LOW→MID, MID→HIGH) | 66.7 · 133.3 (authored absolutes) | tuning `projectile_band_edges` (decoupled from the storey — see below) |
| Aim height | silhouette top × 96⁄175 | tuning `aim_height_frac` (reproduces the old 96 px standing-torso pin) |

Three deliberate decouplings:

- **Band edges ≠ storey-derived.** The clearance edges keep their authored px (66.7 / 133.3) through storey retunes — they were kept when the storey went 200 → 170, because re-deriving thirds-of-storey would shift what projectiles clear (a balance change, not a metric change). Whether they ever re-couple to the storey is an open decision.
- **Cover px ≠ ganger px.** A LOW crate (35) is *shorter* than a prone ganger's silhouette (55) — designed to be shot over. Collapsing both into one band→px table would make the crate unclearable; they stay separate quantities on the same axis.
- **Muzzle heights are authored into the bands**: prone 35 → LOW (can't clear even LOW cover — intended), kneeling 100 → MID, standing 150 → HIGH, so each stance's flat fire leaves in a design-sensible clearance band.

The march itself compares **bands**, not px (`band_for` vs the occupant's height band — see [resolution.md](resolution.md) §2); the continuous px datums feed the *aim* derivation and the authored band assignments. The measured per-part z-bands are parked, unreferenced data — retained for a possible future per-part-geometry pass, not part of the live model.

## Sub-cell precision on the ground plane

The muzzle is a real 3D point inside the shooter's cell: cell center + the per-facing **`barrel_offset_px`** (barrel length 60 px ≈ a third of a cell, along the facing's forward vector; clamped so it can never leave the cell). Model space — decoupled from any view gun-sprite tip, which is the *presenter's* concern. "Up" is +z (`Vec3::Z` in the battle-space basis; distinct from any screen-space orientation the presenter uses).

## Mapping to the view

The presenter (`gdtf_battle_presenter`) owns the projection. The model emits battle-space px (`Vec3` / `IVec2` / `IVec3`); the presenter turns those into world transforms. Conceptually it is two pieces:

- **A single cell↔world bridge** — the only place that maps a cell + storey to a world position and back (`cell_to_world(cell, level)` / `world_to_cell`); gameplay code thinks in `IVec2` cells plus a storey. **TBD (Bevy):** the concrete bridge — whether a Bevy `Resource` holding the grid basis, or a system reading a grid component — is pinned when the presenter implements it.
- **Per-level seating (the storey lift)** — a storey's **lift = `level × z_level_height`** px straight up, read off the **same shared combat tuning** the model marches with (never a local constant), so the lift is **locked 1:1 to the projection**: a model point on storey k's floor (`z = k × z_level_height` battle px) maps exactly onto the world position of `(cell, k)` — trajectories and floors can never drift (to be pinned by a presenter test). Each upper storey's geometry is offset by this same lift, so the model march and the rendered floors land on one plane by construction. **TBD (Bevy):** whether storeys are distinct entity hierarchies offset by a transform, or a shader/layer scheme, is the presenter's call.
- **The view slice (X-COM hard cut)** — every per-level layer above the active view level is **hidden entirely** (no dimming, no transparency). Everything at-or-below renders fully. The roof consequence is intentional: at ground view a roofed building's lid slabs peel away, and anything parented on a hidden level (gangers, props, FX) hides with its layer — raise the view to spot the roof camper. **TBD (Bevy):** likely Bevy `Visibility` toggled per-storey root entity, plus parent-chain inherited visibility — confirmed when the slice lands.
- **Battle→world projection of a trajectory** — projecting a battle-space 3D point (e.g. a shot-outcome trajectory) to world: x/y map through the grid's affine basis; **z maps 1 battle px : 1 unit upward** (which is exactly why the storey lift equals `z_level_height`). Known potential cosmetic seam: the muzzle heights were authored for the clearance bands, so a projected launch point can sit above any gun-sprite tip — a presentation pass, not a model issue.

See [../architecture.md](../architecture.md) for the model/view split this metric sits inside.
