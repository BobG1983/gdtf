use bevy::{
    asset::{AssetEvent, AssetId, Assets},
    image::Image,
    prelude::*,
};

use super::atlases::{SheetRole, TopDownAtlases};
use crate::TerrainFogMaterial;

pub fn redrive_sheet_images_on_asset_event(
    events: Option<MessageReader<AssetEvent<Image>>>,
    atlases: Option<Res<TopDownAtlases>>,
    materials: Option<ResMut<Assets<TerrainFogMaterial>>>,
) {
    let Some(mut events) = events else {
        return;
    };

    let mut modified: Vec<AssetId<Image>> = Vec::new();
    for event in events.read() {
        let AssetEvent::Modified { id } = event else {
            continue;
        };
        if !modified.contains(id) {
            modified.push(*id);
        }
    }
    if modified.is_empty() {
        return;
    }

    if let Some(atlases) = atlases {
        let mut reloaded: Vec<SheetRole> = Vec::new();
        for id in &modified {
            let Some(role) = atlases.sheet_role_for_image(*id) else {
                continue;
            };
            if !reloaded.contains(&role) {
                reloaded.push(role);
            }
        }
        for role in &reloaded {
            info!(
                "tileset hot-reload: reloaded sheet `{}`, refreshing it",
                role.asset_path(),
            );
        }
    }

    let Some(mut materials) = materials else {
        return;
    };
    let stale: Vec<AssetId<TerrainFogMaterial>> = materials
        .iter()
        .filter(|(_, material)| modified.contains(&material.image.id()))
        .map(|(id, _)| id)
        .collect();
    for id in stale {
        if let Some(mut material) = materials.get_mut(id) {
            let _: &mut TerrainFogMaterial = &mut material;
        }
    }
}

pub(crate) fn register_sheet_image_redrive(app: &mut App) {
    app.add_systems(Update, redrive_sheet_images_on_asset_event);
}
