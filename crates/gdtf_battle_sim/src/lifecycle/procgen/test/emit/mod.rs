//! End-to-end emit-step tests (GTW-431 C2/C3; GTW-492 v2 model; GTW-497 by-construction
//! connectivity): the full space-packing pipeline (assemble -> fill -> emit) is
//! DETERMINISTIC under a fixed [`ProcgenRng`] seed (same seed -> identical emitted terrain;
//! different seeds -> different terrain, so the determinism pin is not vacuous), and the
//! emitted [`Situation`] is a VALID assembled level — every cell OUTSIDE a placed region is
//! reachable BY CONSTRUCTION (the 1-cell `default_floor` seam lattice; asserted by flooding
//! the open cells of the REAL packer output — the placed/filled region rectangles — NOT the
//! removed connectivity flood) and every authored cell is in-bounds. That connectivity
//! invariant is proved PIN-DISCRIMINATING by a control (`seam_separated_regions_stay_connected`)
//! that feeds the same flood helper an abutting (seam-less) layout and asserts it splits the
//! board — so the invariant would FAIL if the packer's `Margin::DEFAULT` seam were removed.
//! The REAL pipeline is driven with an injected seeded RNG over the UUID-keyed v2 prefab
//! model ([`PrefabRegistry`] of [`Prefab`]) + the [`UuidThemeRegistry`] (for the theme's
//! default floor) + the [`TerrainDefRegistry`] (classifying each placed piece); the
//! assertions are on the EMITTED output, never on a reimplementation.

mod support;

mod connectivity;
mod determinism;
mod door_stair;
mod wall_orientation;
