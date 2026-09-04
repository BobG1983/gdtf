//! The resource bundles the preview redraw reads, one per job.
use bevy::prelude::*;
use gdtf_battle_presenter::{IsolateView, ViewMode};
use gdtf_battle_sim::{level::UuidThemeRegistry, terrain::def::TerrainDefRegistry};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::{
    canvas::CurrentEditLevel, editor_map::EditorMap, hovered_cell::HoveredCell,
    preview::overlay::PreviewOverlayImages, session::MapEditorSession,
};

/// Whether anything the preview draws changed since the last redraw.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct PreviewDirty(bool);

impl PreviewDirty {
    /// Wraps a change-detection answer.
    const fn new(changed: bool) -> Self {
        Self(changed)
    }
}

impl core::ops::BitOr for PreviewDirty {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 || rhs.0)
    }
}

/// The map being previewed and the cell under the cursor.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct PreviewSubject<'w> {
    pub(super) map:     Option<Res<'w, EditorMap>>,
    pub(super) session: Option<Res<'w, MapEditorSession>>,
    pub(super) hovered: Option<Res<'w, HoveredCell>>,
}

impl PreviewSubject<'_> {
    /// Whether the map, the session, or the hovered cell changed this frame.
    pub(super) fn changed(&self) -> PreviewDirty {
        changed(self.map.as_ref()) | changed(self.session.as_ref()) | changed(self.hovered.as_ref())
    }
}

/// Which storeys the preview draws.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct StoreyVisibility<'w> {
    pub(super) edit_level: Option<Res<'w, CurrentEditLevel>>,
    pub(super) view:       Option<Res<'w, ViewMode>>,
    pub(super) isolate:    Option<Res<'w, IsolateView>>,
}

impl StoreyVisibility<'_> {
    /// Whether the active storey, the storey view mode, or the isolate toggle changed.
    pub(super) fn changed(&self) -> PreviewDirty {
        changed(self.edit_level.as_ref())
            | changed(self.view.as_ref())
            | changed(self.isolate.as_ref())
    }
}

/// The defs, images, and overlay textures a preview tile is drawn from.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct TileArt<'w> {
    pub(super) terrain:      Option<Res<'w, TerrainDefRegistry>>,
    pub(super) themes:       Option<Res<'w, UuidThemeRegistry>>,
    pub(super) sprites:      Option<Res<'w, SpriteDefRegistry>>,
    pub(super) overlays:     Option<Res<'w, PreviewOverlayImages>>,
    pub(super) asset_server: Option<Res<'w, AssetServer>>,
}

impl TileArt<'_> {
    /// Whether any def registry or overlay texture changed this frame.
    pub(super) fn changed(&self) -> PreviewDirty {
        // The asset server ticks as assets stream in, so it never triggers a redraw.
        changed(self.terrain.as_ref())
            | changed(self.themes.as_ref())
            | changed(self.sprites.as_ref())
            | changed(self.overlays.as_ref())
    }
}

// Absent resources count as unchanged; the redraw bails on them separately.
fn changed<T: Resource>(res: Option<&Res<'_, T>>) -> PreviewDirty {
    PreviewDirty::new(res.is_some_and(DetectChanges::is_changed))
}
