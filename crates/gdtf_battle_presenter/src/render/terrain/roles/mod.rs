//! The terrain tile-role seam (GTW-566): the DATA-DRIVEN [`TileRoles`] table (the
//! authored `tile_roles.spritedef.ron` shape + its hot-RON chain) and the presenter's
//! CLOSED [`TileRole`] vocabulary over it — one enum variant per authored role key,
//! the ONE place a graphic-role key is spelled in production Rust.
//!
//! | Submodule | Concern |
//! |-----------|---------|
//! | [`table`] | [`TileIndex`], the [`TileRoles`] resource (serde-tied to the RON), [`TileRoles::index_for_key`], and the GTW-564 hot-RON registration |
//! | [`vocab`] | The [`TileRole`] enum: `as_key` / `from_key` / `index_in` / `def_authorable` / `counterpart` |
//! | `test`    | The key round-trip, vocabulary-completeness, authorable-flag, and counterpart pins |

mod table;
mod vocab;

#[cfg(test)]
mod test;

pub(crate) use table::register_tile_roles_hot_ron;
pub use table::{TileIndex, TileRoles, tile_roles_hot_ron_chain};
pub use vocab::TileRole;
