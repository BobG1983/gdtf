//! Mesh and material assets the terrain draw allocates.

use bevy::{ecs::system::SystemParam, math::primitives::Rectangle, prelude::*};

use crate::TerrainFogMaterial;

/// The shared unit quad and the fog materials terrain tiles are drawn with.
#[derive(SystemParam)]
pub struct TerrainQuads<'w, 's> {
    materials: ResMut<'w, Assets<TerrainFogMaterial>>,
    meshes:    ResMut<'w, Assets<Mesh>>,
    quad:      Local<'s, Option<Handle<Mesh>>>,
}

impl TerrainQuads<'_, '_> {
    /// Unit quad every tile shares, created on first use.
    pub fn quad(&mut self) -> Handle<Mesh> {
        self.quad
            .get_or_insert_with(|| self.meshes.add(Rectangle::from_size(Vec2::ONE)))
            .clone()
    }

    /// Register one tile's fog material and hand back its handle.
    pub fn material(&mut self, material: TerrainFogMaterial) -> Handle<TerrainFogMaterial> {
        self.materials.add(material)
    }
}
