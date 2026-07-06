//! The pooled mutate-in-place route-preview draw: the step / label markers, the
//! disjoint query aliases, the label constants, the draw system, and its spawn helpers.
//!
//! # Mutate, never respawn (the UI-mutate-not-respawn convention)
//!
//! The system maintains a POOL of cell-keyed sprites (and ONE pooled target-cost label): it
//! reuses an existing entity for each route step it now needs (moving its [`Transform`],
//! re-tinting it, showing it) and HIDES surplus pooled entities it no longer needs — it never
//! despawn-then-respawns the set each frame. The walk itself is the shared
//! [`draw_pool`](crate::overlays::pool::draw_pool) helper (GTW-568), which owns the
//! `set_if_neq` visibility flips.

use bevy::{camera::visibility::RenderLayers, prelude::*, text::TextColor};
use gdtf_battle_sim::{CellLevel, Level, SquadVisibility, Tu};

use super::{resolve::preview_draws, seam::PathPreview};
use crate::{
    ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered,
    overlays::pool::draw_pool,
};

/// Marker for a pooled route-preview step [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the same
/// justification the [`HoverHighlight`](crate::HoverHighlight) marker uses):
/// [`draw_path_preview`] queries `With<PathStepSprite>` to find and MUTATE the pooled step
/// sprites in place rather than despawn-respawning them each frame.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PathStepSprite;

/// The pooled route-STEP sprite query for [`draw_path_preview`] — the `(Transform, Sprite,
/// Visibility)` it mutates in place over the [`PathStepSprite`] pool, made `Without` the
/// [`PathTargetLabel`] pool so the two pooled-entity queries are provably disjoint (no B0001
/// query-conflict — `bevy-traps.md` #3 family). A `type` alias so the system signature stays
/// under the `type_complexity` lint. Framework plumbing (a query alias), exempt from
/// no-bare-types.
type StepQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Sprite,
        &'static mut Visibility,
    ),
    (With<PathStepSprite>, Without<PathTargetLabel>),
>;

/// The pooled target-LABEL query for [`draw_path_preview`] — the `(Text2d, Transform,
/// Visibility)` it mutates in place over the SINGLE [`PathTargetLabel`], made `Without` the
/// [`PathStepSprite`] pool so it is provably disjoint from [`StepQuery`] (no B0001
/// query-conflict). A `type` alias so the signature stays under the `type_complexity` lint.
/// Framework plumbing (a query alias), exempt from no-bare-types.
type LabelQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<PathTargetLabel>, Without<PathStepSprite>),
>;

/// Marker for the SINGLE pooled target-cell TU-cost [`Text2d`] label (GTW-368).
///
/// Plumbing around the framework world-space text (the no-bare-types framework carve-out, the
/// same justification the [`PathStepSprite`] marker uses): [`draw_path_preview`] queries
/// `With<PathTargetLabel>` to find and MUTATE the ONE pooled label in place (rewrite its text,
/// move its [`Transform`], show / hide it) rather than despawn-respawning it. Exactly one such
/// entity ever exists — the cost reads on the previewed TARGET cell only, never per-cell.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PathTargetLabel;

/// The colour of the target-cell TU-cost label text — an opaque near-white so the cost reads
/// against the amber route trail beneath it.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`TextColor`], not a domain
/// quantity (the `CELL_PX`-class const carve-out). `pub(super)` so the sibling
/// `path_preview::test` module can pin that the cost TEXT stays FULLY OPAQUE (GTW-371 C1) while
/// the route TILE tint goes ~50% transparent.
pub(super) const LABEL_COLOR: Color = Color::srgb(0.95, 1.0, 0.95);

/// The target-cell TU-cost label font size, in world-space px.
///
/// Framework plumbing — a layout scalar fed straight to a [`TextFont`], not a domain quantity
/// (the `CELL_PX`-class carve-out, the FCT [`spawn_floating_text`](crate::spawn_floating_text)
/// precedent). Small relative to [`CELL_PX`] (`16.0`) so a multi-digit cost fits over one cell.
const LABEL_FONT_PX: f32 = 9.0;

/// The world-space vertical lift of the target-cell TU-cost label ABOVE its cell centre, in
/// world px.
///
/// Lifts the label by just over half a cell so it sits over the top edge of the target cell's
/// route-trail sprite (the FCT / reticle-label "above the cell" treatment).
const LABEL_LIFT_PX: f32 = CELL_PX * 0.55;

/// `Update` ([`PresenterSystems::Overlay`](crate::PresenterSystems)): draw the route
/// path-preview — one cell-keyed [`Sprite`] per route step on the active storey, a MINIMAL
/// marker at the active-storey cell where the route leaves the storey, AND the SINGLE
/// target-cell TU-cost label (C1 / C2 / C3 / C5).
///
/// Reads the presenter-owned [`PathPreview`] read-seam (populated by the input crate, C6),
/// the [`ActiveLevel`], and the sim's [`SquadVisibility`] (the §53 VISIBLE-vs-EXPLORED read,
/// the [`present_fog`](crate::present_fog) precedent), then maintains a POOL of
/// [`PathStepSprite`] sprites plus the ONE [`PathTargetLabel`] cost label:
///
/// - for each route cell ON the active storey, it takes (or lazily spawns) a pooled sprite,
///   moves it to [`cell_to_world_layered`] at the [`Layer::PathPreview`] band, tints it
///   [`PREVIEW_TINT`](super::resolve::PREVIEW_TINT) (full alpha when squad-VISIBLE,
///   [`EXPLORED_ALPHA_SCALE`](super::resolve::EXPLORED_ALPHA_SCALE)-reduced when
///   squad-EXPLORED-but-not-VISIBLE — the §53 remembered treatment), and shows it;
/// - if the route LEAVES the active storey (a later route cell is on a DIFFERENT storey), it
///   draws ONE extra pooled sprite at the LAST active-storey cell tinted
///   [`LINK_MARKER_TINT`](super::resolve::LINK_MARKER_TINT)
///   — the C5 minimal off-storey-continuation marker (GTW-359 soft dep);
/// - every surplus pooled sprite is [`Visibility::Hidden`] — NEVER despawned (the
///   UI-mutate-not-respawn convention, owned by the shared [`draw_pool`] walk).
///
/// # The GTW-368 target-cell cost label (C2)
///
/// It then draws ONE world-space [`Text2d`] showing [`PathPreview::cost`] as `"N TU"` at the
/// previewed TARGET cell — the LAST cell of [`PathPreview::cells`] (the route destination) —
/// lifted [`LABEL_LIFT_PX`] ABOVE the cell ([`draw_target_label`]). The single pooled
/// [`PathTargetLabel`] entity is mutated in place (text rewritten, [`Transform`] moved, shown)
/// — never despawn-respawned. It is HIDDEN when the preview is empty (no target) OR when the
/// target cell is off the active storey (the same hard cut as the steps). So a click sets a
/// target → the route highlights + the cost reads on the destination; clearing the target →
/// the label hides. There are NO per-cell labels (the GTW-357 per-cell labels were dropped).
///
/// Off-[`ActiveLevel`] cells are NOT drawn — the hard cut to the active storey (C5; the
/// GTW-347 fog precedent). Sized to one cell and drawn on
/// [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so it composites with the battlefield,
/// not the GTW-120 UI camera.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the
/// [`PathPreview`] / [`ActiveLevel`] / [`SquadVisibility`] reads, a [`StepQuery`] for the route
/// steps, and a [`LabelQuery`] for the single target-cost label. The two pooled-entity queries
/// are `Without` each other's marker so they are provably disjoint (no B0001 conflict).
/// Battle-gated + in [`PresenterSystems::Overlay`](crate::PresenterSystems) by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin).
pub fn draw_path_preview(
    mut commands: Commands,
    preview: Res<PathPreview>,
    active: Res<ActiveLevel>,
    squad: Res<SquadVisibility>,
    mut steps: StepQuery,
    mut label: LabelQuery,
) {
    let active_level: Level = **active;
    let draws = preview_draws(&preview, active_level, &squad);

    // The world position of a route-step sprite (shared by the reuse + grow paths).
    let world_at =
        |cell: CellLevel| cell_to_world_layered(cell.cell(), active_level, Layer::PathPreview);
    // The shared pooled-draw walk (GTW-568): reuse the pooled step sprites in iteration
    // order (move + re-tint), lazily spawn past the pool, hide the surplus — the helper
    // owns the set_if_neq visibility flips (mutate, not respawn).
    draw_pool(
        steps.iter_mut(),
        draws,
        |draw, (transform, sprite, _)| {
            transform.translation = world_at(draw.cell);
            sprite.color = draw.tint;
        },
        |draw| spawn_step(&mut commands, world_at(draw.cell), draw.tint),
        |(_, _, visibility)| visibility,
    );

    // GTW-368 (C2) — the SINGLE target-cell TU-cost label.
    draw_target_label(&mut commands, &preview, active_level, &mut label);
}

/// Draw (or hide) the SINGLE target-cell TU-cost label for the current preview (GTW-368, C2).
///
/// The target cell is the LAST cell of [`PathPreview::cells`] (the route destination). When the
/// preview is non-empty AND that target cell is on the active storey, the one pooled
/// [`PathTargetLabel`] is moved over it (lifted [`LABEL_LIFT_PX`] ABOVE the cell), its text
/// rewritten to [`label_text`]`(`[`PathPreview::cost`]`)`, and shown — mutated in place (lazily
/// spawned on first need). Otherwise (empty preview / no target, OR a target on a different
/// storey) the label is HIDDEN. Exactly one label entity ever exists — the singleton is the
/// shared [`draw_pool`] walk over a 0/1-length draw list (GTW-568): [`None`] hides the one
/// pooled label via the helper's surplus sweep.
fn draw_target_label(
    commands: &mut Commands,
    preview: &PathPreview,
    active_level: Level,
    label_query: &mut LabelQuery,
) {
    let active_z = i32::from(*active_level);
    // The previewed target = the route destination (the last route cell), shown only when it is
    // on the active storey (the same hard cut as the route steps).
    let target = preview
        .cells()
        .last()
        .copied()
        .filter(|cell| cell.z == active_z);

    // Above the target cell (the FCT / reticle-label "above the cell" treatment).
    let world_at = |cell: CellLevel| {
        let mut world = cell_to_world_layered(cell.cell(), active_level, Layer::PathPreview);
        world.y += LABEL_LIFT_PX;
        world
    };
    draw_pool(
        label_query.iter_mut(),
        target,
        |cell, (label, transform, _)| {
            // Mutate the one pooled label in place (rewrite text, move; the helper shows it).
            let label: &mut Text2d = label;
            **label = label_text(preview.cost());
            transform.translation = world_at(cell);
        },
        |cell| spawn_target_label(commands, label_text(preview.cost()), world_at(cell)),
        |(_, _, visibility)| visibility,
    );
}

/// The target-cell cost label text — the bare TU count followed by `" TU"` (the firemode /
/// status-panel TU convention).
///
/// `pub(super)` so the sibling `path_preview::test` module can pin the label format without an
/// app harness.
pub(super) fn label_text(cost: Tu) -> String {
    format!("{} TU", *cost)
}

/// Lazily spawn the ONE pooled target-cell cost label [`Text2d`] showing `text` at `world`.
///
/// A world-space [`Text2d`] anchored bottom-centre over the cell (so it sits ABOVE the target
/// cell), in [`LABEL_COLOR`] at [`LABEL_FONT_PX`], on the world render layer at the
/// [`Layer::PathPreview`] band. Pooled (kept + reused / hidden, never despawned), so this runs
/// only ONCE — the first time a target is previewed.
fn spawn_target_label(commands: &mut Commands, text: String, world: Vec3) {
    commands.spawn((
        PathTargetLabel,
        Text2d::new(text),
        TextFont {
            font_size: bevy::text::FontSize::Px(LABEL_FONT_PX),
            ..default()
        },
        TextColor(LABEL_COLOR),
        bevy::sprite::Anchor::BOTTOM_CENTER,
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}

/// Lazily spawn ONE pooled route-preview step sprite at `world` tinted `tint`.
///
/// A one-cell translucent [`Sprite`] on the world render layer, shown from spawn. Pooled
/// (kept + reused / re-tinted / hidden, never despawned), so this runs only when the route
/// grows past the current pool size.
fn spawn_step(commands: &mut Commands, world: Vec3, tint: Color) {
    commands.spawn((
        PathStepSprite,
        Sprite {
            color: tint,
            custom_size: Some(Vec2::splat(CELL_PX)),
            ..default()
        },
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
