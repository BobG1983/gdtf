//! Abort-first error tests — a bad vertical link and a missing weapon key each
//! return the typed error and spawn nothing (the no-panic contract).

// GTW-491 (T07a) RETIRED three GTW-396 floor-resolution-against-registry tests
// (`setup_errors_on_a_floor_cost_below_minimum`, `setup_errors_when_floor_key_is_not_a_floor_spec`,
// `floor_override_cost_wins_over_default`): the new `TerrainSimKind` model has no `Floor`
// variant and `TerrainDef` carries no per-piece move cost, so `setup_battle` no longer
// resolves floor move costs against the registry (the floor grid is the uniform
// `fallback_floor_cost`; per-floor move-cost validation returns with the GTW-482
// move-cost-from-`default_floor` work). The behaviour those tests pinned no longer exists in
// this slice, so they were removed rather than left asserting a retired path.

mod equipment;
mod gang;
mod link;
mod stacked;
mod terrain;
