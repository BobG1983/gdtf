//! The [`TerrainFogMaterial`] — a custom [`Material2d`] that renders a terrain atlas tile
//! with a per-instance `saturation` knob (GTW-348).
//!
//! Bevy 0.19's [`Sprite`](bevy::prelude::Sprite) pipeline tints by a per-channel MULTIPLY,
//! which cannot DESATURATE (it has no cross-channel luminance access). The GTW-348 design
//! needs an EXPLORED ("was visible") cell to render at FULL brightness but as GREYSCALE —
//! colour-loss, not brightness-loss, as the memory cue (`docs/combat/visibility.md`
//! §"Tunables"). So the terrain tiles move from the `Sprite` path to this `Material2d` on a
//! shared unit-rect [`Mesh2d`](bevy::sprite::Mesh2d): the WGSL fragment samples the atlas
//! tile, computes its BT.709 luminance, and `mix`es toward greyscale by `(1 - saturation)`
//! (`saturation` `1.0` → full colour, `0.0` → full greyscale).
//!
//! GTW-519 adds a SECOND, orthogonal per-instance knob — `brightness` — for the UFO:EU /
//! `OpenXcom` multi-level display: the active view storey draws full-bright ([`Brightness::FULL`]),
//! a lower drawn storey is DIMMED (`< 1.0`). It plumbs through the SAME material-struct-field →
//! uniform-field → WGSL-binding path as `saturation`, and the shader applies it AFTER the
//! greyscale mix (grey-mix by `saturation`, THEN scale by `brightness`) so the fog colour-loss
//! and the storey-depth darken COMPOSE rather than replace one another.
//!
//! The atlas tile is selected CPU-side: like Bevy's own `SpriteMaterial`, this material's
//! [`AsBindGroupShaderType`] resolves the chosen `atlas_index` against the (resolved)
//! [`TextureAtlasLayout`] into a `uv_transform` matrix, so the atlas index never reaches the
//! GPU — only the baked UV transform does. The presenter mutates `saturation` (and, for a
//! cover-swap, `atlas_index`) in place via [`Assets::get_mut`](bevy::asset::Assets::get_mut),
//! which re-uploads the uniform next frame — never a despawn / respawn (the
//! UI-mutate-not-respawn convention).

use bevy::{
    image::TextureAtlasLayout,
    math::Affine2,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{AsBindGroup, AsBindGroupShaderType, ShaderType},
        texture::GpuImage,
    },
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// The shader-asset path for the terrain-fog [`Material2d`] (vertex + fragment in one file).
const TERRAIN_FOG_SHADER: &str = "shaders/terrain_fog_material.wgsl";

/// The per-tile STOREY-DEPTH brightness scalar the multi-level terrain draw drives
/// (GTW-519).
///
/// A named view-domain newtype (no-bare-types) over the WGSL brightness multiplier: `1.0`
/// draws the tile at its full painted brightness (the ACTIVE view storey), `< 1.0` DIMS it
/// (a lower, drawn-but-non-active storey in the UFO:EU / `OpenXcom` multi-level display). This
/// is a SEPARATE axis from the fog [`saturation`](TerrainFogMaterial::saturation): saturation
/// expresses the squad-VISIBLE / EXPLORED memory cue (colour-loss, never dimmed —
/// `docs/combat/visibility.md`), while brightness expresses the storey-depth of the drawn
/// band. The two COMPOSE in the shader (grey-mix by saturation, THEN scale by brightness),
/// never replace one another. Wraps `f32`; [`Deref`]s to it so the shader plumbing reads the
/// raw multiplier, and it is minted only through [`Brightness::new`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct Brightness(f32);

impl Brightness {
    /// The full-brightness value — the active view storey draws at its painted brightness.
    pub const FULL: Self = Self(1.0);

    /// Build a storey-depth brightness multiplier (`1.0` = full, `< 1.0` = dimmed).
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

impl Default for Brightness {
    /// The seeded default: [`FULL`](Brightness::FULL) — a freshly-drawn tile is full-bright
    /// until the fog writer drives it to a lower storey's dim (mirroring `saturation`'s
    /// VISIBLE seed).
    fn default() -> Self {
        Self::FULL
    }
}

/// The per-tile fog SATURATION scalar the fog writer drives (GTW-348, named for the
/// GTW-627 write seam).
///
/// A named view-domain newtype (no-bare-types) over the WGSL saturation mix factor: `1.0`
/// reads the atlas tile's own colours through unchanged (squad-VISIBLE), `0.0` mixes the
/// tile fully toward its BT.709 luminance (squad-EXPLORED — colour-loss as the memory cue,
/// `docs/combat/visibility.md`). The [`TerrainFogMaterial::saturation`] FIELD stays a bare
/// `f32` (framework material plumbing, documented on the struct); this newtype names the
/// value where it crosses the presenter's own seams. Wraps `f32`; [`Deref`]s to it, minted
/// only through [`Saturation::new`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(crate) struct Saturation(f32);

impl Saturation {
    /// Build a fog saturation mix factor (`1.0` = full colour, `0.0` = full greyscale).
    pub(crate) const fn new(factor: f32) -> Self {
        Self(factor)
    }
}

/// A [`Material2d`] for one terrain tile, with a `saturation` knob the fog writer drives.
///
/// Holds the atlas image, the RESOLVED [`TextureAtlasLayout`] (the layout struct, not the
/// handle — resolved at spawn via [`Assets<TextureAtlasLayout>::get`](bevy::asset::Assets::get)),
/// the chosen `atlas_index`, the tile's `custom_size`, and the `saturation` factor. The
/// uniform sent to the GPU is the [`TerrainFogUniform`] its [`AsBindGroupShaderType`] impl
/// computes — the atlas index is resolved to a UV transform CPU-side and does NOT cross to
/// the shader.
///
/// A framework type (`Asset` / `Material2d`), so its fields are the material plumbing the
/// derive macros require rather than no-bare-types domain values: the wrapped domain
/// quantities live in the sim's tuning ([`ViewRange`](gdtf_battle_sim::tuning::CombatTuning) /
/// [`ExploredDim`](gdtf_battle_sim::tuning::CombatTuning)); `saturation` here is the rendered
/// expression of the VISIBLE / EXPLORED decision the fog writer makes.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
#[uniform(0, TerrainFogUniform)]
pub struct TerrainFogMaterial {
    /// The terrain sheet image (the same handle the sprite path used).
    #[texture(1)]
    #[sampler(2)]
    pub image:        Handle<Image>,
    /// The RESOLVED atlas layout over [`Self::image`] (the layout struct, resolved at spawn
    /// from the handle via [`Assets<TextureAtlasLayout>::get`](bevy::asset::Assets::get)).
    /// [`None`] when the layout was not loaded — the UV transform stays the identity (the
    /// whole image), so the spawn caller skips a tile it cannot resolve.
    pub atlas_layout: Option<TextureAtlasLayout>,
    /// The atlas tile index (into `atlas_layout.textures`). Mutated by the cover-swap.
    pub atlas_index:  usize,
    /// The rendered tile size in world units (matches the sprite path's `custom_size`).
    pub custom_size:  Option<Vec2>,
    /// `1.0` = full colour (VISIBLE), `0.0` = full greyscale (EXPLORED). Mutated by the fog
    /// writer in place each frame.
    pub saturation:   f32,
    /// The GTW-519 storey-depth brightness: [`Brightness::FULL`] on the ACTIVE view storey,
    /// a dimmed `< 1.0` on a lower drawn storey (the UFO:EU multi-level darken). A SEPARATE
    /// axis from [`saturation`](Self::saturation) — it COMPOSES with it in the shader
    /// (grey-mix by saturation, then scale by brightness), never replaces it. Mutated by the
    /// fog writer in place each frame (per-storey), the same in-place path as `saturation`.
    pub brightness:   Brightness,
}

/// The GPU uniform for [`TerrainFogMaterial`] (binding 0).
///
/// Mirrors Bevy's `SpriteMaterialUniform` layout shape: the atlas UV transform plus the
/// quad scale, with the GTW-348 `saturation` and the GTW-519 `brightness` added. The `Mat3`
/// field aligns to a 48-byte block (three `vec4` columns) under [`ShaderType`]; the three
/// trailing scalars pack EXACTLY into the next 16-byte boundary (`vertex_scale` 8 +
/// `saturation` 4 + `brightness` 4 = 16 bytes), so no explicit pad is needed and the Rust
/// and WGSL layouts agree. (GTW-519 folded the GTW-348 `pad` slot into `brightness`.)
#[derive(ShaderType, Default)]
pub struct TerrainFogUniform {
    /// Maps the unit-rect UV to the atlas tile's UV rect (baked from `atlas_index` +
    /// `atlas_layout` CPU-side).
    pub uv_transform: Mat3,
    /// The quad size in world units (the vertex shader scales the unit rect by this).
    pub vertex_scale: Vec2,
    /// `1.0` = full colour, `0.0` = full greyscale (the fog EXPLORED colour-loss axis).
    pub saturation:   f32,
    /// `1.0` = full brightness (active storey), `< 1.0` = dimmed (a lower drawn storey — the
    /// GTW-519 UFO:EU multi-level darken). Fills the byte that was the GTW-348 `pad`; the
    /// shader multiplies the (grey-mixed) RGB by this AFTER the saturation mix, so the two
    /// axes compose.
    pub brightness:   f32,
}

impl AsBindGroupShaderType<TerrainFogUniform> for TerrainFogMaterial {
    /// Resolve the atlas tile rect to a UV transform (mirroring Bevy's `SpriteMaterial`) and
    /// pack the quad scale + `saturation` into the GPU uniform.
    ///
    /// Returns the [`Default`] uniform (identity transform, zero scale, opaque) while the
    /// image's GPU texture is not yet prepared — the same first-frame fallback
    /// `SpriteMaterial` uses; the next frame re-derives it once the texture lands.
    fn as_bind_group_shader_type(&self, images: &RenderAssets<GpuImage>) -> TerrainFogUniform {
        let Some(image) = images.get(self.image.id()) else {
            return TerrainFogUniform::default();
        };
        let image_size = image.size_2d().as_vec2();
        let mut uv_transform = Affine2::default();

        if let Some(layout) = &self.atlas_layout {
            // Clamp the index into the layout (an out-of-range index reads the last tile
            // rather than panicking); an empty layout leaves the identity transform.
            let last = layout.textures.len().saturating_sub(1);
            let index = self.atlas_index.min(last);
            if let Some(tile) = layout.textures.get(index) {
                let rect = tile.as_rect();
                let size = rect.size();
                // Guard a degenerate zero-size rect (a malformed layout) — the divide below
                // would NaN the UV; fall back to the identity (the whole image).
                if size.x > 0.0 && size.y > 0.0 && image_size.x > 0.0 && image_size.y > 0.0 {
                    uv_transform *= Affine2::from_scale(size / image_size);
                    uv_transform *= Affine2::from_translation(Vec2::new(
                        rect.min.x / size.x,
                        rect.min.y / size.y,
                    ));
                }
            }
        }

        let vertex_scale = self.custom_size.unwrap_or(image_size);

        TerrainFogUniform {
            uv_transform: uv_transform.into(),
            vertex_scale,
            saturation: self.saturation,
            brightness: *self.brightness,
        }
    }
}

impl Material2d for TerrainFogMaterial {
    fn vertex_shader() -> ShaderRef {
        TERRAIN_FOG_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        TERRAIN_FOG_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        // Mask at 0.5 so a fully-transparent atlas margin discards (the tiles are opaque
        // tile art with transparent gaps) rather than blending a halo.
        AlphaMode2d::Mask(0.5)
    }
}
