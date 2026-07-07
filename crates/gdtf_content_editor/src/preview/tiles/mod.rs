//! The prefab preview-viewport **tile sprites** (GTW-515 C4.3 / C4.4; GTW-594 C2) — the
//! change-driven redraw that populates the offscreen preview with a sprite per drawable
//! cell, classified per storey through the presenter's ONE shared
//! [`storey_treatment`](gdtf_battle_presenter::storey_treatment) classifier and rendered
//! through the editor's own treatment table, plus the translucent HOVER GHOST (red when
//! the placement is illegal).
//!
//! Every sprite is a world-space [`Sprite`](bevy::prelude::Sprite) on the isolated
//! preview render layer with the `PreviewTile` marker, positioned by the shared
//! cell↔world mapping — so the dedicated preview camera renders these and ONLY these
//! (the `RenderLayers` isolation, C4.3). The redraw is CHANGE-DRIVEN (despawn-all +
//! respawn) when any input the preview depends on changes; it reads the SAME model +
//! resolution the save path + the capture drive read, so the preview shows exactly what a
//! save would write.
//!
//! Wiring-only module: the change-driven redraw system lives in [`redraw`]; the treatment
//! table (tints / overlay patterns / z layout), the sprite-spawn helpers, and the hover
//! ghost live in [`sprites`].

mod redraw;
mod sprites;

#[cfg(test)]
mod test;

pub(crate) use redraw::redraw_preview_tiles;
