//! Unit tests for the unified terrain-definition model (GTW-484). Wiring only:
//! `mod` declarations, no test bodies.
//!
//! - [`parse`] — named-struct RON parse per `sim_kind` + the `presenter_kind` footfall
//!   shape (C1, C3, C5).
//! - [`tags`] — sim-owned tags default-empty + round-trip, on the sim side (C2).
//! - [`round_trip`] — `deserialize(serialize(def)) == def` per kind, identity only (C4).
//! - [`registry`] — insert-by-key + lookup-by-`TerrainUuid` (C7).
//! - [`derive_blocking`] — GTW-501: the path-blocking DERIVATION rule (Wall/Cover block by
//!   default, Slab does not, an explicit `BlocksPathfinding` tag adds blocking).
//! - [`derive_vision`] — GTW-502: the vision-occlusion DERIVATION rule (Wall/Cover occlude at
//!   their band by default, Slab does not, an explicit `BlocksVision` tag adds occlusion).
//! - [`kind_projection`] — GTW-574: the canonical [`TerrainPieceKind`](crate::terrain::entity::TerrainPieceKind)
//!   projections (every sim/presenter variant projects onto its piece kind) + the
//!   `ALL` inventory completeness pin.

mod derive_blocking;
mod derive_vision;
mod kind_projection;
mod parse;
mod registry;
mod round_trip;
mod tags;
