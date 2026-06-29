//! The terrain **piece** identity newtypes — [`TerrainName`] (a terrain file's
//! filename stem), [`TerrainGraphicKey`] (the opaque presenter-resolved graphic
//! role key), and [`FootfallSound`] (the opaque presenter-resolved footfall key).
//!
//! These shared, render-free identity / presentation-hook newtypes are reused by the
//! UUID-keyed terrain-definition model ([`def`](super::def)) and the presenter. The
//! legacy per-file terrain authoring struct + its name-keyed registry resource were deleted
//! by GTW-496 (child T10 of the GTW-476 data-model refactor) once the UUID model
//! ([`TerrainDef`](super::def::TerrainDef) / [`TerrainDefRegistry`](super::def::TerrainDefRegistry))
//! became the sole terrain model on every live path.
//!
//! Mirrors the `equipment/weapon` dir-module layout (memory:
//! *code-health-module-layout*): `mod.rs` is wiring-only; per-concern files carry
//! the types.

mod components;

pub use components::{FootfallSound, TerrainGraphicKey, TerrainName};
