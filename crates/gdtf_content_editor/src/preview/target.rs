//! the preview camera + EVERY preview tile sit on [`PREVIEW_LAYER`] (`RenderLayers::layer(1)`),
use bevy::{
    camera::{
        Camera, ClearColorConfig, Projection, RenderTarget, ScalingMode, visibility::RenderLayers,
    },
    prelude::*,
    render::render_resource::TextureFormat,
};

use crate::{
    canvas::CanvasZoom,
    preview::{coords::PREVIEW_VIEW_SPAN, view::PreviewPan},
};

/// [`PREVIEW_TEXTURE_EDGE`]²-px image (a legible fixed resolution; the view SPAN, not this pixel
pub(crate) const PREVIEW_TEXTURE_EDGE: u32 = 512;

const PREVIEW_LAYER: usize = 1;

const PREVIEW_CLEAR: Color = Color::srgb(0.09, 0.10, 0.12);

#[derive(Resource, Debug, Clone)]
pub struct PreviewTarget {
        image: Handle<Image>,
}

impl PreviewTarget {
        #[must_use]
    pub(crate) fn image_handle(&self) -> Handle<Image> {
        self.image.clone()
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PreviewCamera;

/// system despawns + respawns each change, all on [`PREVIEW_LAYER`]. A no-bare-types unit marker.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PreviewTile;

#[must_use]
pub(crate) const fn preview_layer() -> RenderLayers {
    RenderLayers::layer(PREVIEW_LAYER)
}

/// spawn the dedicated preview [`Camera2d`] rendering into it on the isolated [`PREVIEW_LAYER`]
pub(crate) fn spawn_preview_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    egui_user_textures: Option<ResMut<bevy_egui::EguiUserTextures>>,
) {
    let image = Image::new_target_texture(
        PREVIEW_TEXTURE_EDGE,
        PREVIEW_TEXTURE_EDGE,
        TextureFormat::Rgba8UnormSrgb,
        Some(TextureFormat::Rgba8UnormSrgb),
    );
    let handle = images.add(image);

    if let Some(mut egui_user_textures) = egui_user_textures {
        egui_user_textures.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone()));
    }
    commands.insert_resource(PreviewTarget {
        image: handle.clone(),
    });

    // A Fixed-span orthographic projection so the visible world area is PREVIEW_VIEW_SPAN² world
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width:  PREVIEW_VIEW_SPAN,
        height: PREVIEW_VIEW_SPAN,
    };

    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(PREVIEW_CLEAR),
            ..default()
        },
        Projection::Orthographic(projection),
        RenderTarget::Image(handle.into()),
        preview_layer(),
        PreviewCamera,
    ));
}

pub(crate) fn despawn_preview_target(
    mut commands: Commands,
    cameras: Query<Entity, With<PreviewCamera>>,
    tiles: Query<Entity, With<PreviewTile>>,
) {
    for entity in &cameras {
        commands.entity(entity).despawn();
    }
    for entity in &tiles {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<PreviewTarget>();
}

pub(crate) fn apply_preview_view(
    zoom: Option<Res<CanvasZoom>>,
    pan: Option<Res<PreviewPan>>,
    mut camera: Query<(&mut Projection, &mut Transform), With<PreviewCamera>>,
) {
    let (Some(zoom), Some(pan)) = (zoom, pan) else {
        return;
    };
    let Ok((mut projection, mut transform)) = camera.single_mut() else {
        return;
    };
    if let Projection::Orthographic(ortho) = &mut *projection {
        ortho.scale = **zoom;
    }
    let offset = pan.offset();
    transform.translation.x = offset.x;
    transform.translation.y = offset.y;
}
