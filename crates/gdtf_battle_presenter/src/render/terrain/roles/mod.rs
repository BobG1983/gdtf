//! The terrain tile-role seam (GTW-566): the presenter's CLOSED [`TileRole`]
//! vocabulary — one variant per authorable graphic-role key, the ONE place a
//! graphic-role key is spelled in production Rust.
//!
//! GTW-665 retired the role→atlas-index TABLE that used to live beside it
//! (`TileRoles` + the `tile_roles.spritedef.ron` hot-RON chain): a graphic key
//! now resolves through the sprite-def registry
//! ([`resolve_sprite`](super::resolve::resolve_sprite)). The ENUM stays the
//! closed renderer vocabulary/concern seam — the sim-fact → role fallback
//! mapping, the editor's authorable-picker filter, and the prefab connector
//! pairing all still classify through it.
//!
//! | Submodule | Concern |
//! |-----------|---------|
//! | [`vocab`] | The [`TileRole`] enum: `as_key` / `from_key` / `def_authorable` / `counterpart` |
//! | `test`    | The key round-trip, seeded-catalog lockstep, authorable-flag, and counterpart pins |

mod vocab;

#[cfg(test)]
mod test;

pub use vocab::TileRole;
