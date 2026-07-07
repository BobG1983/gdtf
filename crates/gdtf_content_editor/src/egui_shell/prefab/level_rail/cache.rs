//! The rail's **change-keyed thumbnail cache** (GTW-595 C2) — one retained egui texture
//! per storey, rebuilt ONLY when that storey's [`StoreySignature`] changes.
//!
//! The signature is a pure fold of the storey's rendered content (painted cells +
//! resolved hues + grid extent — [`storey_key`](super::occupancy::storey_key)), so the
//! cache is keyed on CONTENT rather than on `is_changed` ticks: the egui shell holds the
//! [`EditorMap`] through `ResMut` and deref-muts it every prefab frame (the viewport's
//! paint path), which marks the resource changed whether or not a paint landed — a
//! tick-keyed rebuild would therefore fire every frame. Content keying honours the
//! CLAUSE (rebuild only when the map or roles actually change) and is idempotent under
//! the egui multipass re-run for free: the second pass probes the same signature and
//! reuses the texture the first pass stored (bevy-traps #8 fact (b)).

use bevy::platform::collections::HashMap;
use bevy_egui::egui;
use gdtf_battle_sim::{level::GridSize, metric::Level};

use super::{
    occupancy::{PaintedCount, StoreySignature},
    scrub::ScrubAccumulator,
};

/// One storey's cached rail row content: the change-key it was built under, the painted
/// count the row label shows, and the retained egui texture its thumbnail draws.
pub(crate) struct RailThumb {
    /// The [`StoreySignature`] the texture was built from — a differing probe rebuilds.
    pub(super) signature: StoreySignature,
    /// The storey's painted-cell count at build time (signature-covered, so a cache hit
    /// implies this is current).
    pub(super) count:     PaintedCount,
    /// The retained egui texture the row's thumbnail [`egui::Image`] draws. Dropping it
    /// frees the texture (egui's handle semantics), so pruning an entry releases GPU
    /// memory too.
    pub(super) texture:   egui::TextureHandle,
}

/// The per-storey thumbnail cache (GTW-595 C2) — the change-keyed [`egui::ColorImage`]
/// store behind the rail's rows. Held as a `Local` of the shell system (view-local UI
/// state, the `prefab_save_name` precedent): content keying makes it self-correcting, so
/// it needs no state-scoped lifecycle of its own.
#[derive(Default)]
pub(crate) struct RailThumbCache {
    /// The cached rows, keyed by storey.
    thumbs: HashMap<Level, RailThumb>,
}

impl RailThumbCache {
    /// The cached thumb for `storey`, REBUILT through `build` only when `signature`
    /// differs from the stored one (or no thumb exists yet) — the C2 change-keyed
    /// rebuild. A rebuild over an existing entry re-uploads IN PLACE
    /// ([`egui::TextureHandle::set`] — same texture id, no flicker); a first build loads
    /// a fresh retained texture. Returns [`None`] only in the unreachable
    /// just-inserted-then-missing case (the lookup is total after an insert; no panic
    /// path per the workspace lints).
    pub(crate) fn refresh(
        &mut self,
        ctx: &egui::Context,
        storey: Level,
        signature: StoreySignature,
        count: PaintedCount,
        build: impl FnOnce() -> egui::ColorImage,
    ) -> Option<&RailThumb> {
        let stale = self
            .thumbs
            .get(&storey)
            .is_none_or(|thumb| thumb.signature != signature);
        if stale {
            let image = build();
            if let Some(thumb) = self.thumbs.get_mut(&storey) {
                thumb.texture.set(image, egui::TextureOptions::NEAREST);
                thumb.signature = signature;
                thumb.count = count;
            } else {
                let texture = ctx.load_texture(
                    format!("gdtf_level_rail_l{}", *storey),
                    image,
                    egui::TextureOptions::NEAREST,
                );
                self.thumbs.insert(
                    storey,
                    RailThumb {
                        signature,
                        count,
                        texture,
                    },
                );
            }
        }
        self.thumbs.get(&storey)
    }

    /// Drop every cached storey at or past the grid's level count — a level-count shrink
    /// must release the vanished storeys' textures (idempotent; a no-op while the extent
    /// is unchanged).
    pub(super) fn prune(&mut self, size: GridSize) {
        let levels = i32::from(*size.levels());
        self.thumbs.retain(|storey, _| i32::from(**storey) < levels);
    }

    /// How many storeys currently hold a cached thumb — the prune/rebuild observability
    /// hook for the cache-keying tests.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.thumbs.len()
    }
}

/// The rail's whole view-local UI state — the thumbnail cache plus the wheel-scrub
/// accumulator, bundled so the shell threads ONE `Local` (GTW-595). Not a domain value:
/// a plumbing aggregate of the rail's retained view state.
#[derive(Default)]
pub(crate) struct RailUiState {
    /// The change-keyed per-storey thumbnail cache (C2).
    pub(super) thumbs: RailThumbCache,
    /// The wheel-scrub remainder fold (C1 scroll-to-scrub).
    pub(super) scrub:  ScrubAccumulator,
}
