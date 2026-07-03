//! The per-tile terrain ECS entity layer — one entity per authored terrain piece
//! (wall / scatter cover / floor–roof slab), spawned by [`setup_battle`](crate::situation::setup_battle)
//! and indexed for query-by-cell via [`TerrainIndex`]. GTW-395.
//!
//! ## Scope
//!
//! Path B (from-ledger spawn): terrain entities are spawned directly from the
//! already-populated source iterators (`situation.walls` / `situation.scatter` for
//! cover, `situation.slabs` for slabs) during `setup_battle`'s existing pour — the
//! same transaction that seeds the [`CoverLedger`](crate::cover::CoverLedger),
//! [`SlabLedger`](crate::slab::SlabLedger), and [`SurfaceGrid`](crate::surface::SurfaceGrid).
//!
//! ## Components on each entity
//!
//! - [`TerrainCell`] — the `(cell, level)` position (the queryable cell key).
//! - [`TerrainPieceKind`] — `Wall` / `Cover` / `Slab` / `Emplacement` (the queryable
//!   kind tag, and since GTW-574 the CANONICAL terrain-kind discriminant the payload
//!   enums project onto via their `kind()` projections).
//! - Cover entities additionally carry [`CoverHp`](crate::cover::CoverHp) (max),
//!   [`HeightBand`](crate::cover::HeightBand), [`ArmorProtection`](crate::armor::ArmorProtection),
//!   [`ArmorHardness`](crate::armor::ArmorHardness) — static stats copied from the
//!   authored [`CoverSpawn`](crate::situation::CoverSpawn).
//! - Slab entities carry [`SlabHp`](crate::slab::SlabHp) (max),
//!   [`ArmorProtection`](crate::armor::ArmorProtection),
//!   [`ArmorHardness`](crate::armor::ArmorHardness) — seeded from the `SlabEntry`
//!   prototype resolved in `setup_battle_on_request`.
//!
//! ## Design authority
//!
//! The entity's `max_hp` component is the **static ceiling** — it never changes.
//! Live current HP is authoritative in the [`CoverLedger`](crate::cover::CoverLedger) /
//! [`SlabLedger`](crate::slab::SlabLedger). The bridge pattern (C3): a system gets
//! the entity from [`TerrainIndex`], reads `max_hp` off the component, and reads
//! live HP from the ledger by the same `CellLevel` key.

pub mod components;
pub mod index;

#[cfg(test)]
pub(crate) mod test;

pub use components::{
    BlocksPathfinding, BlocksVision, TerrainBrace, TerrainCell, TerrainPieceKind,
};
pub use index::{TerrainIndex, TerrainIndexKey};
