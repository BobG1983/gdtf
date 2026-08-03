use bevy::platform::collections::HashMap;
use bevy_egui::egui;
use gdtf_battle_sim::{level::GridSize, metric::Level};

use super::{
    occupancy::{PaintedCount, StoreySignature},
    scrub::ScrubAccumulator,
};

pub(crate) struct RailThumb {
        pub(super) signature: StoreySignature,
            pub(super) count:     PaintedCount,
                pub(super) texture:   egui::TextureHandle,
}

#[derive(Default)]
pub(crate) struct RailThumbCache {
        thumbs: HashMap<Level, RailThumb>,
}

impl RailThumbCache {
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

                pub(super) fn prune(&mut self, size: GridSize) {
        let levels = i32::from(*size.levels());
        self.thumbs.retain(|storey, _| i32::from(**storey) < levels);
    }

            #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.thumbs.len()
    }
}

#[derive(Default)]
pub(crate) struct RailUiState {
        pub(super) thumbs: RailThumbCache,
        pub(super) scrub:  ScrubAccumulator,
}
