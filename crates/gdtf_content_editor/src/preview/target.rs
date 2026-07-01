//! The prefab preview-viewport **render target + offscreen camera** (GTW-515 C4.3) — the
//! `RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_DST` [`Image`] the preview draws INTO, the DEDICATED
//! second [`Camera2d`] that renders the preview tiles into it on an ISOLATED
//! [`RenderLayers`], the egui texture registration, and the once-per-frame set-to-target
//! [`apply_preview_view`] that drives the camera from the owned zoom/pan.
//!
//! ## Isolation approach shipped: `RenderLayers` (the research-recommended path)
//!
//! Following the RESEARCH REFERENCE (the shipping `bevy_egui` `render_to_image_widget.rs` pattern):
//! the preview camera + EVERY preview tile sit on [`PREVIEW_LAYER`] (`RenderLayers::layer(1)`),
//! while the editor's own window camera + everything else stay on the default layer 0. The two are
//! then mutually invisible, so the preview camera renders ONLY the prefab preview tiles (never the
//! window/UI content) and the window camera never renders the preview tiles. The editor's existing
//! primary egui context (auto-created on the window `Camera2d`) is UNTOUCHED — the preview is just a
//! texture drawn inside that primary context (research note (e)), so NO second egui context /
//! `PrimaryEguiContext` / `auto_create_primary_context = false` is needed.
//!
//! ## The multipass-idempotent apply (bevy-traps #8 fact (b))
//!
//! [`apply_preview_view`] runs in `Update` (NOT the egui closure), once per frame, and SETS the
//! camera's [`OrthographicProjection::scale`] + [`Transform::translation`] to the owned target
//! ([`CanvasZoom`] + [`PreviewPan`]) — a set-to-target assignment, never an accumulate. The egui
//! wheel/pan handler only ever writes those owned resources (also set-to-target), so a multipass
//! re-run of the closure can never double-apply the camera mutation.

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

/// The offscreen render target's square edge in pixels — the preview draws into a
/// [`PREVIEW_TEXTURE_EDGE`]²-px image (a legible fixed resolution; the view SPAN, not this pixel
/// size, governs how much world is visible). A framework layout const (the no-bare-types clause-4
/// plumbing carve-out).
pub(crate) const PREVIEW_TEXTURE_EDGE: u32 = 512;

/// The dedicated render layer the preview camera + every preview tile live on (research-recommended
/// isolation). Layer `1` — disjoint from the editor window camera's default layer `0`. A framework
/// plumbing const.
const PREVIEW_LAYER: usize = 1;

/// The preview viewport's CLEAR colour — a dark slate so an empty viewport is visibly the preview
/// (not a black void that could be mistaken for a render failure — C4.12: never ship a black
/// viewport). A framework layout const.
const PREVIEW_CLEAR: Color = Color::srgb(0.09, 0.10, 0.12);

/// The offscreen **preview render target** (GTW-515 C4.3) — the [`Image`] handle the preview
/// camera renders into and egui draws as its viewport widget.
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). The handle is a framework plumbing value (no-bare-types clause-4 carve-out); the
/// resource keeps a STRONG handle alive so the render target + the egui-registered texture outlive
/// any transient owner. Read by the egui viewport draw ([`image_handle`](PreviewTarget::image_handle))
/// and by [`apply_preview_view`] indirectly (the camera holds its own [`RenderTarget::Image`]).
#[derive(Resource, Debug, Clone)]
pub struct PreviewTarget {
    /// The `RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_DST` image the preview renders into.
    image: Handle<Image>,
}

impl PreviewTarget {
    /// The offscreen image handle — the egui viewport registers + draws this.
    #[must_use]
    pub(crate) fn image_handle(&self) -> Handle<Image> {
        self.image.clone()
    }
}

/// Marker on the dedicated offscreen preview camera (GTW-515 C4.3) — so [`apply_preview_view`]
/// drives THIS camera's projection/transform, never the editor's window camera. A no-bare-types
/// unit marker.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PreviewCamera;

/// Marker on a spawned preview TILE sprite (GTW-515 C4.3 / C4.4) — the entities the tiles-redraw
/// system despawns + respawns each change, all on [`PREVIEW_LAYER`]. A no-bare-types unit marker.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PreviewTile;

/// The [`RenderLayers`] every preview entity (camera + tiles) is spawned on — the isolation layer.
#[must_use]
pub(crate) const fn preview_layer() -> RenderLayers {
    RenderLayers::layer(PREVIEW_LAYER)
}

/// `OnEnter(Editing)`: create the offscreen [`PreviewTarget`] image, register it with egui, and
/// spawn the dedicated preview [`Camera2d`] rendering into it on the isolated [`PREVIEW_LAYER`]
/// (GTW-515 C4.3).
///
/// The image uses [`Image::new_target_texture`], which sets exactly the
/// `RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_DST` usages a render target needs. The camera has
/// `order: -1` so the offscreen pass runs BEFORE the window pass, a [`ClearColorConfig::Custom`]
/// dark-slate clear (never black — C4.12), a [`ScalingMode::Fixed`] orthographic projection
/// (a fixed world span independent of the texture pixel size — deterministic UV↔world mapping),
/// and the [`PreviewCamera`] marker. Param-only (bevy-traps #7): [`Commands`] + the two
/// asset/egui resources. The [`EguiUserTextures`](bevy_egui::EguiUserTextures) borrow is `Option`
/// so the target + camera still spawn under the headless test harness (which runs no
/// [`EguiPlugin`](bevy_egui::EguiPlugin), so the resource is absent) — only the egui image
/// registration no-ops there; the windowed binary always has it.
pub(crate) fn spawn_preview_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    egui_user_textures: Option<ResMut<bevy_egui::EguiUserTextures>>,
) {
    // The render-target image: new_target_texture sets RENDER_ATTACHMENT | TEXTURE_BINDING |
    // COPY_DST and zero-fills the buffer. Rgba8UnormSrgb matches the format bevy_egui specializes
    // its pipeline to — a Bgra8UnormSrgb target trips a wgpu "Incompatible color attachments"
    // validation error against `egui_pipeline` (the RenderPass texture format must match the
    // pipeline's), observed intermittently in the capture harness; Rgba8UnormSrgb avoids it.
    let image = Image::new_target_texture(
        PREVIEW_TEXTURE_EDGE,
        PREVIEW_TEXTURE_EDGE,
        TextureFormat::Rgba8UnormSrgb,
        Some(TextureFormat::Rgba8UnormSrgb),
    );
    let handle = images.add(image);

    // Register the image with egui ONCE (idempotent per AssetId). A STRONG handle — the editor also
    // owns it via the RenderTarget + the PreviewTarget resource, so keeping egui's alive is fine.
    // Absent under the headless harness (no EguiPlugin) — the target still spawns; only registration
    // is skipped.
    if let Some(mut egui_user_textures) = egui_user_textures {
        egui_user_textures.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone()));
    }
    commands.insert_resource(PreviewTarget {
        image: handle.clone(),
    });

    // A Fixed-span orthographic projection so the visible world area is PREVIEW_VIEW_SPAN² world
    // units at scale 1, independent of the texture's pixel resolution (deterministic mapping).
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

/// `OnExit(Editing)`: despawn the preview camera + remove the [`PreviewTarget`] (the
/// state-scoped-resource pattern — bevy-traps #1). The offscreen [`Image`] asset is dropped when
/// the last strong handle (this resource + the camera's `RenderTarget`) goes; the egui-registered
/// weak entry is cleaned up by `bevy_egui`'s own free system.
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

/// `Update` (in `Editing`): drive the preview camera from the owned zoom + pan targets, SET-TO-
/// TARGET (GTW-515 C4.7 / C4.8; bevy-traps #8 fact (b)).
///
/// Runs ONCE per frame OUTSIDE the egui closure, so the camera mutation can never double-apply
/// under the egui multipass re-run. Assigns the [`OrthographicProjection::scale`] from
/// [`CanvasZoom`] and the [`Transform::translation`] from [`PreviewPan`] — never accumulates. All
/// borrows are `Option` (state-scoped — bevy-traps #1); no-ops until the resources + camera exist.
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
        // SET the scale (never *=): the owned CanvasZoom is the authoritative target.
        ortho.scale = **zoom;
    }
    // SET the translation (never +=): the owned PreviewPan is the authoritative target.
    let offset = pan.offset();
    transform.translation.x = offset.x;
    transform.translation.y = offset.y;
}
