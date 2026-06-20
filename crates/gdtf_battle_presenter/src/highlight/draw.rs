//! The hover-highlight message, the marker, and the request-driven draw system.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{Cell, CellLevel, Level};

use crate::{CELL_PX, WORLD_RENDER_LAYER, cell_to_world};

/// A request to move/show/hide the hover-highlight on a cell.
///
/// The presenter-owned input API for the highlight: a buffered [`Message`] (Bevy
/// 0.18 — buffered events are messages, `bevy-traps.md` #4) carrying the cell to
/// highlight, or [`None`] to clear/hide it. A NAMED single-field newtype over the
/// sim's [`CellLevel`] (no-bare-types — the requested cell is a domain value),
/// [`Deref`]ing to its inner [`Option`] so the draw system matches it directly.
///
/// The INPUT crate WRITES this (via [`MessageWriter`] after its picker resolves the
/// hovered cell); [`draw_highlight_on_request`] READS it. Keeping the payload to the
/// cell this slice means the future gamepad cursor (GTW-259) reuses the SAME pipeline
/// — it emits the same request from the stick-driven pick.
#[derive(Message, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightRequest(Option<CellLevel>);

impl HighlightRequest {
    /// Build a highlight request from the cell to highlight, or [`None`] to hide it.
    #[must_use]
    pub const fn new(cell: Option<CellLevel>) -> Self {
        Self(cell)
    }
}

/// Marker for the single hover-highlight [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the
/// same justification the presenter's [`WorldCamera`](crate::WorldCamera) /
/// [`TerrainSprite`](crate::TerrainSprite) markers use): [`draw_highlight_on_request`]
/// queries `With<HoverHighlight>` to find and MOVE the one existing highlight rather
/// than spawning a duplicate each request.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct HoverHighlight;

/// The translucent tint of the hover-highlight sprite.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a
/// domain quantity (the `CELL_PX`-class const carve-out). A solid-color reticle is
/// the engineer's-choice highlight visual: there is no reticle role in the landed
/// atlas (only Terrain / Characters / Effects), so a tinted solid `Sprite` is the
/// renderer-agnostic option. A faint warm-white at low alpha so it reads as a
/// highlight OVER the cell without hiding the tile beneath. MIGRATED verbatim from
/// `gdtf_battle_input` (GTW-251) so the look is unchanged.
const HIGHLIGHT_TINT: Color = Color::srgba(1.0, 0.95, 0.6, 0.35);

/// Maintains exactly ONE hover-highlight sprite from the latest [`HighlightRequest`].
///
/// Drains the [`MessageReader<HighlightRequest>`] and acts on the LAST request this
/// update (the freshest cursor read — if the picker emitted more than once in a
/// frame, only the newest matters). Spawns the single [`HoverHighlight`] sprite the
/// first time a request carries [`Some`] cell; on every later request it MOVES that
/// one sprite's [`Transform`] to [`cell_to_world`]`(cell)` and shows it
/// ([`Visibility::Visible`]) for [`Some`], or HIDES it ([`Visibility::Hidden`]) for
/// [`None`] — so no duplicate highlight sprites accumulate. The sprite is sized to
/// exactly one cell (`custom_size: Some(Vec2::splat(CELL_PX))`, the S3/S4 sizing
/// recipe) and drawn on [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so it
/// composites with the battlefield, not the GTW-120 UI camera.
///
/// When NO request arrives this update the highlight is left untouched (it keeps
/// following the last requested cell), so the picker — which emits every update it
/// runs — keeps the drawn highlight in lockstep with the hovered cell.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy spawn, a
/// [`MessageReader<HighlightRequest>`] for the request, and a
/// `Query<(&mut Transform, &mut Visibility), With<HoverHighlight>>` for the
/// move + show/hide. Battle-gated + in [`PresenterSystems::Draw`](crate::PresenterSystems)
/// by the [`TopDownRendererPlugin`](crate::TopDownRendererPlugin), so it observes a
/// settled sim state and is inert pre-battle.
pub fn draw_highlight_on_request(
    mut commands: Commands,
    mut requests: MessageReader<HighlightRequest>,
    mut highlights: Query<(&mut Transform, &mut Visibility), With<HoverHighlight>>,
) {
    // Act on only the LATEST request this update — earlier reads are stale cursor
    // positions superseded by the freshest one. No request => leave the highlight as
    // it is (it keeps tracking the last requested cell).
    let Some(request) = requests.read().last().copied() else {
        return;
    };

    // The world position the one highlight should take, or `None` to hide it.
    let target = (*request)
        .map(|cell| cell_to_world(Cell::new(cell.x, cell.y), Level::new(level_index(cell))));

    match highlights.single_mut() {
        Ok((mut transform, mut visibility)) => match target {
            Some(world) => {
                transform.translation = world;
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        },
        // No highlight yet: spawn the single sprite the first time a cell is requested.
        // (A `None`-only request before any hover has nothing to spawn — it stays
        // absent until the first `Some`, which is equivalent to "hidden".)
        Err(_) => {
            if let Some(world) = target {
                commands.spawn((
                    HoverHighlight,
                    Sprite {
                        color: HIGHLIGHT_TINT,
                        custom_size: Some(Vec2::splat(CELL_PX)),
                        ..default()
                    },
                    Transform::from_translation(world),
                    // Explicitly Visible (not the `Inherited` default) so the reticle
                    // shows from the first frame it is requested, independent of any
                    // parent visibility.
                    Visibility::Visible,
                    RenderLayers::layer(WORLD_RENDER_LAYER),
                ));
            }
        }
    }
}

/// The storey index of a [`CellLevel`]'s `z`, narrowed to the [`Level`]'s `u8`.
///
/// A [`CellLevel`] [`Deref`]s to an `IVec3` whose `z` is a storey index built from a
/// [`Level`] (always `0..`[`gdtf_battle_sim::MAX_LEVELS`], well within `u8`). The
/// clamped [`u8::try_from`] is the no-`unwrap` narrow — a (impossible) out-of-range
/// `z` saturates to [`u8::MAX`] rather than panicking. MIGRATED from
/// `gdtf_battle_input` (GTW-251) so the highlight redraws on the requested level.
pub(super) fn level_index(cell: CellLevel) -> u8 {
    u8::try_from(cell.z).unwrap_or(u8::MAX)
}
