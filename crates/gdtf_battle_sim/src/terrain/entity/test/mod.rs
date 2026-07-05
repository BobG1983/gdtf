//! GTW-395 acceptance tests — the per-tile terrain ECS entity layer.
//!
//! Each test corresponds to a named acceptance clause in the GTW-395 design:
//!
//! - **Test 1**: One entity per terrain piece spawned (C1).
//! - **Test 2**: Entity carries static stats from authored data (C2).
//! - **Test 3**: Cell-to-entity lookup via [`TerrainIndex`] (C4 / queryable-by-cell).
//! - **Test 5**: Index + ledger lifetime — both [`TerrainIndex`] AND [`SlabLedger`] are
//!   absent after teardown (blocker 1 / pre-existing leak fix).
//! - **Test 6**: Bridge pattern — [`TerrainIndex`] → entity → [`CoverHp`] (max); live HP
//!   from [`CoverLedger`] by the same key (C3).
//! - **Test 7**: Typed-key no-collision — a cover and a slab at the same [`CellLevel`]
//!   produce TWO distinct index entries (major-4 fix).

mod support;

mod components;
mod index;
mod projection;
mod teardown;
