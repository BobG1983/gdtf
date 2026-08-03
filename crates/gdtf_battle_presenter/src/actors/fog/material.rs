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

const TERRAIN_FOG_SHADER: &str = "shaders/terrain_fog_material.wgsl";

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct Brightness(f32);

impl Brightness {
        pub const FULL: Self = Self(1.0);

        #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

impl Default for Brightness {
                fn default() -> Self {
        Self::FULL
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(crate) struct Saturation(f32);

impl Saturation {
        pub(crate) const fn new(factor: f32) -> Self {
        Self(factor)
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
#[uniform(0, TerrainFogUniform)]
pub struct TerrainFogMaterial {
        #[texture(1)]
    #[sampler(2)]
    pub image:        Handle<Image>,
                    pub atlas_layout: Option<TextureAtlasLayout>,
        pub atlas_index:  usize,
        pub custom_size:  Option<Vec2>,
            pub saturation:   f32,
                        pub brightness:   Brightness,
}

#[derive(ShaderType, Default)]
pub struct TerrainFogUniform {
            pub uv_transform: Mat3,
        pub vertex_scale: Vec2,
        pub saturation:   f32,
                    pub brightness:   f32,
}

impl AsBindGroupShaderType<TerrainFogUniform> for TerrainFogMaterial {
                            fn as_bind_group_shader_type(&self, images: &RenderAssets<GpuImage>) -> TerrainFogUniform {
        let Some(image) = images.get(self.image.id()) else {
            return TerrainFogUniform::default();
        };
        let image_size = image.size_2d().as_vec2();
        let mut uv_transform = Affine2::default();

        if let Some(layout) = &self.atlas_layout {
            let last = layout.textures.len().saturating_sub(1);
            let index = self.atlas_index.min(last);
            if let Some(tile) = layout.textures.get(index) {
                let rect = tile.as_rect();
                let size = rect.size();
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
        AlphaMode2d::Mask(0.5)
    }
}
