//! The editor-owned `Failed -> default` fallback for the tile-role chain.
//!
//! ADR-0003 (GTW-579 C4b): every asset the editor's `Load` pass gates on must
//! fall back to a const default on a genuine `Failed`, so the editor NEVER
//! hangs in `Load` on a bad/missing file. The presenter
//! ships NO [`TileRoles`] fallback (the game leaves the resource absent and its
//! gate waits), so the zero table below is the editor's own policy — attached
//! at the editor's chain registration ([`register_load`](super::register_load)
//! via [`HotRonChain::with_fallback`](gdtf_assets::HotRonChain::with_fallback)),
//! never a mode flag in the generic seam or the presenter's config.

use gdtf_battle_presenter::{TileIndex, TileRoles};

/// The fallback [`TileRoles`] table — every role at index `0` — used only when
/// `sprites/tile_roles.spritedef.ron` fails to load, so a bad asset never
/// strands the editor in `Load`. A degraded but non-panicking table (every
/// terrain then draws the sheet's first tile).
pub(crate) const fn default_tile_roles() -> TileRoles {
    let zero = TileIndex::new(0);
    TileRoles {
        floor:                zero,
        floor_alt_panel:      zero,
        wall:                 zero,
        wall_ew:              zero,
        cover:                zero,
        emplacement:          zero,
        emplacement_occupied: zero,
        slab:                 zero,
        rubble:               zero,
        slab_destroyed:       zero,
        door:                 zero,
        stair_up:             zero,
        stair_down:           zero,
        ladder:               zero,
        door_ns:              zero,
        door_ew:              zero,
        stair_ns_up:          zero,
        stair_ns_down:        zero,
        stair_ew_up:          zero,
        stair_ew_down:        zero,
    }
}
