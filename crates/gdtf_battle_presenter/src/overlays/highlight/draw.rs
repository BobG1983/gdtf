//! The hover-highlight message, the marker, and the request-driven draw system.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::prelude::CellLevel;

use crate::{CELL_PX, CellVisibility, WORLD_RENDER_LAYER, cell_to_world};

/// A request to move/show/hide the hover-highlight on a cell, CARRYING the squad-visible
/// verdict so the reticle recolours into a non-VISIBLE cell (GTW-11).
///
/// The presenter-owned input API for the highlight: a buffered [`Message`] (Bevy
/// 0.18 — buffered events are messages, `bevy-traps.md` #4) carrying the cell to
/// highlight (or [`None`] to clear/hide it) ALONGSIDE the cell's [`CellVisibility`]
/// verdict (the squad-visible gate, `docs/combat/visibility.md` §"UX edges"). Both fields
/// are NAMED domain values (no-bare-types: the requested cell is a [`CellLevel`], the
/// verdict is a [`CellVisibility`], not a bare bool). It [`Deref`]s to its cell
/// [`Option`] so the draw system reads the cell directly; the verdict is read through
/// [`visibility`](HighlightRequest::visibility).
///
/// The INPUT crate WRITES this (via [`MessageWriter`] after its picker resolves the
/// hovered cell AND the shared
/// [`cell_squad_visible`](crate::cell_squad_visible) verdict);
/// [`draw_highlight_on_request`] READS it and tints the one reticle sprite by the
/// carried verdict. The future gamepad cursor (GTW-259) reuses the SAME pipeline — its
/// pick flows through the same emitter, which computes the same verdict.
#[derive(Message, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightRequest {
    /// The cell to highlight, or [`None`] to hide the reticle. [`Deref`] target so
    /// existing reads (`*request`) keep matching the cell.
    #[deref]
    cell:       Option<CellLevel>,
    /// The cell's squad-visible verdict — drives the reticle tint (a non-VISIBLE cell
    /// recolours to the [`UNSEEN_TINT`]).
    visibility: CellVisibility,
}

impl HighlightRequest {
    /// Build a highlight request from the cell to highlight (or [`None`] to hide it)
    /// and its squad-visible [`CellVisibility`] verdict.
    #[must_use]
    pub const fn new(cell: Option<CellLevel>, visibility: CellVisibility) -> Self {
        Self { cell, visibility }
    }

    /// The cell's squad-visible verdict — the [`CellVisibility`] the reticle tint
    /// branches on (the carried gate result, so reticle + hint + fire-refusal agree).
    #[must_use]
    pub const fn visibility(self) -> CellVisibility {
        self.visibility
    }
}

/// Marker for the single hover-highlight [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the
/// same justification the presenter's [`WorldCamera`](crate::WorldCamera) /
/// [`TerrainSprite`](crate::TerrainSprite) markers use): [`draw_highlight_on_request`]
/// queries `With<HoverHighlight>` to find and MOVE the one existing highlight rather
/// than spawning a duplicate each request.
///
/// The [`Default`] is the `bsn!` spawn-seed sentinel (GTW-322): every inline
/// component-position form in a `bsn!` scene seeds its slot via `Default::default()`
/// before the value is written; a unit marker's `Default` is trivial and carries no
/// authored state.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct HoverHighlight;

/// The translucent tint of the hover-highlight sprite.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a
/// domain quantity (the `CELL_PX`-class const carve-out). A solid-color reticle is
/// the engineer's-choice highlight visual: there is no reticle role in the landed
/// atlas (only Terrain / Characters / Effects), so a tinted solid `Sprite` is the
/// renderer-agnostic option. A faint warm-white at low alpha so it reads as a
/// highlight OVER the cell without hiding the tile beneath. MIGRATED verbatim from
/// `gdtf_battle_input` (GTW-251) so the look is unchanged. Used for a squad-VISIBLE
/// cell ([`CellVisibility::SquadVisible`]) — the normal "you can act here" reticle.
const HIGHLIGHT_TINT: Color = Color::srgba(1.0, 0.95, 0.6, 0.35);

/// The reticle tint over a NON-VISIBLE cell ([`CellVisibility::NotSquadVisible`], UNSEEN
/// or merely EXPLORED) — the "unseen — hold your fire" recolour (GTW-11,
/// `docs/combat/visibility.md` §"UX edges"). A desaturated cold-grey at the SAME alpha as
/// [`HIGHLIGHT_TINT`] so it reads as "the reticle is here but you may not act" — visually
/// DISTINCT from the warm-white normal tint (drained of warmth, the colour-loss fog cue
/// mirroring GTW-348's EXPLORED greyscale). Framework plumbing — a literal [`Color`] handed
/// to a [`Sprite`] (the `CELL_PX`-class const carve-out), not a domain quantity.
const UNSEEN_TINT: Color = Color::srgba(0.55, 0.6, 0.7, 0.35);

/// The reticle tint for a [`CellVisibility`] verdict: a squad-VISIBLE cell keeps the warm
/// [`HIGHLIGHT_TINT`]; a non-VISIBLE cell (UNSEEN / EXPLORED) recolours to [`UNSEEN_TINT`].
const fn tint_for(visibility: CellVisibility) -> Color {
    if visibility.is_squad_visible() {
        HIGHLIGHT_TINT
    } else {
        UNSEEN_TINT
    }
}

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
/// GTW-11 — the reticle RECOLOURS off the request's carried
/// [`CellVisibility`](crate::CellVisibility) verdict: a squad-VISIBLE cell keeps the warm
/// `HIGHLIGHT_TINT`, a non-VISIBLE cell (UNSEEN / EXPLORED) takes the `UNSEEN_TINT`
/// ("unseen — hold your fire"). It STILL mutates the ONE [`HoverHighlight`] sprite in
/// place (its `color` alongside its `Transform` / `Visibility`) — never a second sprite.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy spawn, a
/// [`MessageReader<HighlightRequest>`] for the request, and a
/// `Query<(&mut Transform, &mut Sprite, &mut Visibility), With<HoverHighlight>>` for the
/// move + recolour + show/hide. Battle-gated + in [`PresenterSystems::Overlay`](crate::PresenterSystems)
/// by the [`TopDownRendererPlugin`](crate::TopDownRendererPlugin), so it observes a
/// settled sim state and is inert pre-battle.
pub fn draw_highlight_on_request(
    mut commands: Commands,
    mut requests: MessageReader<HighlightRequest>,
    mut highlights: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<HoverHighlight>>,
) {
    // Act on only the LATEST request this update — earlier reads are stale cursor
    // positions superseded by the freshest one. No request => leave the highlight as
    // it is (it keeps tracking the last requested cell).
    let Some(request) = requests.read().last().copied() else {
        return;
    };

    // The world position the one highlight should take, or `None` to hide it — the
    // canonical CellLevel::split decompose (GTW-565).
    let target = (*request).map(|cell| {
        let (cell, level) = cell.split();
        cell_to_world(cell, level)
    });
    // The tint for this request's squad-visible verdict (warm = VISIBLE, cold-grey =
    // non-VISIBLE — the "unseen — hold your fire" recolour).
    let tint = tint_for(request.visibility());

    match highlights.single_mut() {
        Ok((mut transform, mut sprite, mut visibility)) => match target {
            Some(world) => {
                transform.translation = world;
                // Recolour the ONE sprite in place off the carried verdict.
                sprite.color = tint;
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        },
        // No highlight yet: spawn the single sprite the first time a cell is requested.
        // (A `None`-only request before any hover has nothing to spawn — it stays
        // absent until the first `Some`, which is equivalent to "hidden".)
        Err(_) => {
            if let Some(world) = target {
                let sprite = Sprite {
                    // Spawn already tinted by the carried verdict (so the first-frame
                    // reticle recolours correctly, not only on a later move).
                    color: tint,
                    custom_size: Some(Vec2::splat(CELL_PX)),
                    ..default()
                };
                let transform = Transform::from_translation(world);
                // Explicitly Visible (not the `Inherited` default) so the reticle
                // shows from the first frame it is requested, independent of any
                // parent visibility.
                let visibility = Visibility::Visible;
                let layers = RenderLayers::layer(WORLD_RENDER_LAYER);
                // GTW-322 — authored as a `bsn!` scene (mirrors the converted ganger
                // sprite). The `Sprite` is NOT `Unpin` (its `Option<Handle<Image>>` /
                // `Option<TextureAtlas>` fields), so it rides NEITHER `template_value`
                // (which bounds `Unpin`) nor a `bsn!` field patch — it takes the
                // `template(move |_| Ok(value.clone()))` closure escape hatch (the
                // `FnTemplate` has no `Unpin` bound on its output). The `Transform` /
                // `Visibility` / `RenderLayers` ARE `Clone + Default + Unpin`, so each
                // rides `template_value`. The `HoverHighlight` marker is a unit type, so
                // it inlines in the macro.
                commands.spawn_scene((
                    bsn! {
                        HoverHighlight
                        template(move |_| Ok(sprite.clone()))
                    },
                    template_value(transform),
                    template_value(visibility),
                    template_value(layers),
                ));
            }
        }
    }
}
