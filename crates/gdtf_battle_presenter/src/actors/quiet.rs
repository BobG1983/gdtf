use bevy::{
    asset::{AssetId, Assets},
    prelude::{DetectChangesMut, Mut, ResMut, Visibility},
};

use super::fog::{Brightness, Saturation, TerrainFogMaterial};

pub(super) fn set_visibility_quiet(visibility: &mut Mut<Visibility>, target: Visibility) {
    visibility.set_if_neq(target);
}

pub(super) fn set_fog_knobs_quiet(
    materials: &mut ResMut<Assets<TerrainFogMaterial>>,
    id: AssetId<TerrainFogMaterial>,
    saturation: Saturation,
    brightness: Brightness,
) {
    let differs = materials.get(id).is_some_and(|material| {
        material.saturation.to_bits() != saturation.to_bits()
            || material.brightness.to_bits() != brightness.to_bits()
    });
    if !differs {
        return;
    }
    if let Some(mut material) = materials.get_mut(id) {
        material.saturation = *saturation;
        material.brightness = brightness;
    }
}
