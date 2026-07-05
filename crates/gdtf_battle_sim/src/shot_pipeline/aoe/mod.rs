//! The `AoE` **hit-template resolver** — the pure, render-free function that enumerates
//! the set of affected `(cell, level)` a shot's [`HitType`](crate::weapon::HitType)
//! template covers at its impact cell (GTW-541, CORE of GTW-41).
//!
//! `docs/combat/combat.md` (the one-line blast-radii note — "blast radii ... assume
//! square tiles") is the only `docs/` anchor for area weapons on the square grid; the
//! template GEOMETRY below (Chebyshev disc, cone half-angle wedge, line beam, 2D-on-level,
//! flat falloff) is defined HERE, not in `docs/`. After the `resolution.md` §2 ray-march
//! resolves an impact cell, an `AoE` weapon does not strike
//! just that cell's occupant — it applies its template (a blast disc, a cone wedge, or a
//! line beam) and strikes EVERY occupant in the covered set. This module owns the
//! GEOMETRY of those templates as a pure function over the sim's cell metric; the live
//! fire path ([`fire`](crate::fire::fire)) reads the returned set, queries each cell's
//! [`occupant`](crate::occupancy::OccupancyGrid::occupant), and routes each struck entity
//! through the EXISTING damage pipeline (it reimplements no damage math).
//!
//! ## The templates (GTW-541)
//!
//! - [`HitType::Single`](crate::weapon::HitType::Single) — the impact cell only (the
//!   unchanged pre-GTW-541 single-target path; the live fire path never even calls this
//!   resolver for `Single`, but the resolver returns the singleton set for completeness).
//! - [`HitType::Blast`](crate::weapon::HitType::Blast)`{ radius }` — an
//!   **omnidirectional** disc: every cell within `radius` (Chebyshev — the square grid's
//!   natural king-move distance) of the impact cell, **on the impact's own storey**.
//!   Radius `0` = the impact cell only.
//! - [`HitType::Cone`](crate::weapon::HitType::Cone)`{ range, angle }` — a **directed
//!   wedge** whose apex is the SHOOTER: every cell within `range` cells (Chebyshev) of the
//!   impact whose bearing from the shooter is within `angle` degrees of the shooter→impact
//!   fire direction (plus the impact cell itself), on the impact's storey.
//! - [`HitType::Line`](crate::weapon::HitType::Line)`{ range }` — a **beam**: the impact
//!   cell plus up to `range` cells stepping the shooter→impact ground direction, on the
//!   impact's storey.
//!
//! ## Design rulings (GTW-541 — logged, defensible defaults)
//!
//! - **Blast is 2D-on-level, not a 3D ball.** The coarse model bands VERTICAL exposure by
//!   silhouette height (§1/§2), and a square-grid blast template is a HORIZONTAL footprint
//!   (`docs/combat/combat.md`: "blast radii ... assume square tiles"). So the disc lies on
//!   the impact's own storey; a multi-storey spherical blast is a later tuning axis, not
//!   this CORE. The same 2D-on-level rule applies to the cone wedge and the line beam.
//! - **FLAT damage across the whole set — no distance falloff.** Every affected cell takes
//!   the full shot's damage (the logged GTW-541 default); a falloff curve is a later ticket.
//! - **Determinism.** The returned set is SORTED canonically (`(z, y, x)` — the sim's
//!   `cell_key` order), so the live path resolves struck entities in a stable order and the
//!   seeded RNG stream is byte-stable across runs. This function itself takes NO RNG draw.
//! - **Friendly fire hits ALL occupants** — the resolver enumerates CELLS, faction-blind;
//!   the fire path strikes every occupant it finds (`docs/combat/resolution.md` §2: "any
//!   other actor in the path — including your own gang — true friendly fire"). Grenades do
//!   not discriminate. The set INCLUDES the shooter's own cell if the geometry covers it.
//!
//! Pure model logic: no systems, no `&mut World`, no ECS trigger, no pixel. Every cell is
//! clamped to the [`GRID_WIDTH`](crate::occupancy::GRID_WIDTH) ×
//! [`GRID_HEIGHT`](crate::occupancy::GRID_HEIGHT) bounds so an edge-impact template never
//! yields an off-grid cell (edge-clamp, GTW-541 edge case).

mod resolve;

#[cfg(test)]
mod test;

pub use resolve::aoe_affected;
