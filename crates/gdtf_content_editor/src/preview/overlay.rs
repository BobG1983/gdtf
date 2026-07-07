//! The preview's GENERATED overlay textures (GTW-594 C2) — the 2×2-px-block stipple/checker
//! sheet the categorical below-ghost lays over context tiles, and the faint void-grid cell
//! outline unpainted ACTIVE cells render — built procedurally once per `Editing` span (no
//! authored asset; the patterns are structural, not art).

use bevy::{
    asset::RenderAssetUsages,
    image::{Image, ImageSampler},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

/// The stipple sheet's pixel edge — an 8×8 texture of 2×2-px checker blocks, stretched over
/// a preview cell (nearest-sampled, so the blocks stay crisp). The stretch factor sets the
/// on-screen stipple scale — flagged for in-engine zoom tuning (GTW-594 C2, view-ticket QA
/// canon). A framework layout const (clause-4 plumbing carve-out).
const STIPPLE_PX: u32 = 8;

/// The stipple's checker block edge in texture pixels (the "2×2 stipple" — GTW-594 C2).
const STIPPLE_BLOCK_PX: u32 = 2;

/// The void-grid sheet's pixel edge — one preview cell at the presenter's native 16 px.
const VOID_GRID_PX: u32 = 16;

/// An opaque white RGBA texel — patterns are authored WHITE and tinted per-sprite (the
/// same tint path every preview tile sprite rides).
const WHITE_TEXEL: [u8; 4] = [255, 255, 255, 255];

/// A fully transparent RGBA texel — the pattern's "off" pixels.
const CLEAR_TEXEL: [u8; 4] = [0, 0, 0, 0];

/// The generated preview-overlay texture handles (GTW-594 C2), built ONCE per `Editing`
/// span by [`create_preview_overlay_images`] and read by the preview redraw.
///
/// A named [`Resource`] (a framework type; the handles it holds are framework plumbing,
/// read through named accessors). State-scoped by hand (inserted `OnEnter(Editing)`,
/// removed `OnExit(Editing)` — bevy-traps #1) because building the images needs
/// [`Assets<Image>`], which the shared seed-closure seam cannot reach.
#[derive(Resource, Debug, Clone)]
pub(crate) struct PreviewOverlayImages {
    /// The 2×2-px-block checker sheet the below-ghost stipple overlay draws.
    stipple:   Handle<Image>,
    /// The faint cell-outline sheet an unpainted ACTIVE cell draws (the void grid).
    void_grid: Handle<Image>,
}

impl PreviewOverlayImages {
    /// The stipple/checker sheet handle — the categorical below-ghost overlay.
    pub(crate) fn stipple(&self) -> Handle<Image> {
        self.stipple.clone()
    }

    /// The void-grid cell-outline sheet handle — the unpainted-active-cell marker.
    pub(crate) fn void_grid(&self) -> Handle<Image> {
        self.void_grid.clone()
    }
}

/// `OnEnter(Editing)`: build the two generated overlay textures and insert
/// [`PreviewOverlayImages`] (GTW-594 C2). Param-only (`bevy-traps.md` #7): [`Commands`] +
/// the [`Assets<Image>`] store.
pub(crate) fn create_preview_overlay_images(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    commands.insert_resource(PreviewOverlayImages {
        stipple:   images.add(stipple_image()),
        void_grid: images.add(void_grid_image()),
    });
}

/// `OnExit(Editing)`: drop the overlay-texture resource (the handles die with it, freeing
/// the generated images) — the manual half of the state-scoped lifecycle.
pub(crate) fn remove_preview_overlay_images(mut commands: Commands) {
    commands.remove_resource::<PreviewOverlayImages>();
}

/// Build a white-on-transparent pattern texture from a per-pixel predicate: `on(x, y)`
/// texels are opaque white (tinted per-sprite at spawn), the rest fully transparent.
/// Nearest-sampled so the pattern stays crisp when a sprite stretches it over a cell.
fn pattern_image(edge: u32, on: impl Fn(u32, u32) -> bool) -> Image {
    let mut data = Vec::with_capacity((edge * edge * 4) as usize);
    for y in 0..edge {
        for x in 0..edge {
            let texel = if on(x, y) { WHITE_TEXEL } else { CLEAR_TEXEL };
            data.extend_from_slice(&texel);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width:                 edge,
            height:                edge,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// The stipple/checker sheet (GTW-594 C2): alternating [`STIPPLE_BLOCK_PX`]-square blocks —
/// on/off in a checkerboard — so the below-ghost reads as a texture-coded class (not just a
/// tint) at any zoom.
fn stipple_image() -> Image {
    pattern_image(STIPPLE_PX, |x, y| {
        ((x / STIPPLE_BLOCK_PX) + (y / STIPPLE_BLOCK_PX)).is_multiple_of(2)
    })
}

/// The void-grid sheet (GTW-594 C2): a one-pixel outline on the cell's left + top edges —
/// adjacent cells tile the lines into a faint grid over the empty active storey.
fn void_grid_image() -> Image {
    pattern_image(VOID_GRID_PX, |x, y| x == 0 || y == 0)
}
