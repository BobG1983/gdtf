//! The fire-target highlight (GTW-371 · C2): the presenter-owned read-seam
//! [`FireTargetHighlight`] resource (the hovered fireable-enemy cell + the fire TU cost), its
//! draw system ([`draw_fire_target`] — the RED under-actor tile + the OPAQUE TU-cost label),
//! and the single pooled tile / label markers.
//!
//! # The C2 read-seam (input → presenter → sim)
//!
//! The PRESENTER owns the read-seam ([`FireTargetHighlight`], the hovered fireable-enemy
//! [`CellLevel`] + the [`Tu`] fire cost) plus this draw system; the INPUT crate (which alone
//! may read `SelectedShooter` + `SelectedFireMode` + the hovered cell) decides the fireable
//! verdict (mirroring `decide_left_click`'s FIRE rung), computes the cost via
//! [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost), and POPULATES this resource (clearing it
//! off any non-fireable hover). Selection / fire-mode / hover NEVER enter the sim, and the
//! dependency direction stays `input → presenter → sim` — the SAME shape as the
//! [`PathPreview`](crate::PathPreview) seam.
//!
//! # Under the actor (the contract's "rendered UNDER the enemy sprite")
//!
//! The red tile draws at [`Layer::FireTarget`](crate::Layer) (`z` strictly below the
//! [`Actor`](crate::Layer::Actor) band), so it composites UNDER the enemy's own sprite. The
//! OPAQUE TU-cost label (the move-cost-label treatment) lifts above the cell so it reads.
//!
//! # Hard-cut to the active storey (the GTW-347 fog / GTW-358 preview precedent)
//!
//! The draw hard-cuts to the presenter's [`ActiveLevel`](crate::ActiveLevel): a fire target on
//! the active storey is shown; an off-storey target (or no target) HIDES the tile + label.
//!
//! # Mutate, never respawn (the UI-mutate-not-respawn convention)
//!
//! The affordance is a SINGLE cell, so the system maintains ONE pooled tile [`Sprite`] and ONE
//! pooled label [`Text2d`]: it moves / re-shows them on a fireable hover and HIDES them
//! otherwise — it never despawn-then-respawns. Each singleton is the shared
//! [`draw_pool`](crate::overlays::pool::draw_pool) walk (GTW-568) over a 0/1-length draw list;
//! the helper owns the `set_if_neq` visibility flips.

use bevy::{camera::visibility::RenderLayers, prelude::*, text::TextColor};
use gdtf_battle_sim::{Cell, CellLevel, Level, Tu};

use crate::{
    ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered,
    overlays::pool::draw_pool,
};

/// The presenter-owned fire-target highlight read-seam — the hovered fireable-enemy cell the
/// SELECTED shooter could fire on, plus the fire TU cost (C2).
///
/// A named domain value (no-bare-types: the fire-target affordance is a domain value), holding
/// an `Option<(CellLevel, Tu)>`: [`Some`] when the cursor hovers a fireable enemy (the cell + the
/// [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost) the fire would charge), [`None`] when not
/// hovering a fireable enemy (empty cell / own ganger / non-visible enemy / no selection). OWNED
/// BY THE PRESENTER so the `input → presenter → sim` direction holds: the draw system READS it;
/// the input crate's `populate_fire_target` POPULATES it (the
/// [`HighlightRequest`](crate::HighlightRequest) precedent). `init_resource`-d by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin), so its [`Default`] is the empty
/// highlight (no fireable hover → nothing drawn).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FireTargetHighlight {
    /// The hovered fireable-enemy cell + the fire TU cost, or [`None`] when not hovering a
    /// fireable enemy.
    target: Option<(CellLevel, Tu)>,
}

impl FireTargetHighlight {
    /// Build a highlight on `cell` with the fire `cost` — what the input populate writes when
    /// the cursor hovers a fireable enemy.
    #[must_use]
    pub const fn new(cell: CellLevel, cost: Tu) -> Self {
        Self {
            target: Some((cell, cost)),
        }
    }

    /// The empty (no fireable hover) highlight — the [`Default`], and what the input populate
    /// writes when the cursor is not over a fireable enemy.
    #[must_use]
    pub const fn cleared() -> Self {
        Self { target: None }
    }

    /// The hovered fireable-enemy cell, if any — [`None`] when not hovering a fireable enemy.
    #[must_use]
    pub const fn cell(&self) -> Option<CellLevel> {
        match self.target {
            Some((cell, _)) => Some(cell),
            None => None,
        }
    }

    /// The fire TU cost on the hovered cell, if any — exactly the
    /// [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost) a fire would charge.
    #[must_use]
    pub const fn cost(&self) -> Option<Tu> {
        match self.target {
            Some((_, cost)) => Some(cost),
            None => None,
        }
    }

    /// Whether the highlight is empty (not hovering a fireable enemy → nothing drawn).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.target.is_none()
    }
}

/// Marker for the SINGLE pooled fire-target RED tile [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the same
/// justification the [`PathStepSprite`](crate::PathStepSprite) marker uses): [`draw_fire_target`]
/// queries `With<FireTargetTile>` (and `Without<FireTargetLabel>`) to find and MUTATE the ONE
/// pooled tile in place (move it, show / hide it) rather than despawn-respawning it.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct FireTargetTile;

/// Marker for the SINGLE pooled fire-target TU-cost [`Text2d`] label.
///
/// Plumbing around the framework world-space text (the no-bare-types framework carve-out, the
/// same justification the [`PathTargetLabel`](crate::PathTargetLabel) marker uses):
/// [`draw_fire_target`] queries `With<FireTargetLabel>` (and `Without<FireTargetTile>`) to find
/// and MUTATE the ONE pooled label in place (rewrite its text, move it, show / hide it).
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct FireTargetLabel;

/// The pooled RED-TILE query for [`draw_fire_target`] — the `(Transform, Visibility)` it mutates
/// in place over the SINGLE [`FireTargetTile`], made `Without` the [`FireTargetLabel`] so the two
/// pooled-entity queries are provably disjoint (no B0001 query-conflict — `bevy-traps.md` #3
/// family). A `type` alias so the system signature stays under the `type_complexity` lint.
/// Framework plumbing (a query alias), exempt from no-bare-types.
type TileQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (With<FireTargetTile>, Without<FireTargetLabel>),
>;

/// The pooled COST-LABEL query for [`draw_fire_target`] — the `(Text2d, Transform, Visibility)`
/// it mutates in place over the SINGLE [`FireTargetLabel`], made `Without` the [`FireTargetTile`]
/// so it is provably disjoint from [`TileQuery`] (no B0001 query-conflict). A `type` alias so the
/// signature stays under the `type_complexity` lint. Framework plumbing (a query alias), exempt
/// from no-bare-types.
type LabelQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<FireTargetLabel>, Without<FireTargetTile>),
>;

/// The RED tint of the fire-target tile — a red at ~50% alpha so the enemy / terrain beneath it
/// reads through (FLAGGED engineer-choice, GTW-371 C2: the user said transparency applies to "just
/// the path", but the fire-target tile sits UNDER the enemy sprite, so semi-transparent keeps the
/// enemy legible; trivially flipped to opaque by raising this alpha to `1.0`).
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a domain quantity
/// (the `CELL_PX`-class const carve-out, the [`PREVIEW_TINT`](crate::path_preview) precedent).
const FIRE_TARGET_TINT: Color = Color::srgba(1.0, 0.15, 0.1, 0.5);

/// The colour of the fire-cost label text — an OPAQUE near-white (C2: the cost label is opaque,
/// like the move-cost label), so the cost reads against the red tile + the enemy beneath it.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`TextColor`], not a domain
/// quantity (the `CELL_PX`-class const carve-out, the move-cost
/// [`LABEL_COLOR`](crate::path_preview) precedent).
const COST_LABEL_COLOR: Color = Color::srgb(0.95, 1.0, 0.95);

/// The fire-cost label font size, in world-space px — small relative to [`CELL_PX`] (`16.0`) so a
/// multi-digit cost fits over one cell (the move-cost-label precedent).
///
/// Framework plumbing — a layout scalar fed straight to a [`TextFont`], not a domain quantity.
const COST_LABEL_FONT_PX: f32 = 9.0;

/// The world-space vertical lift of the fire-cost label ABOVE its cell centre, in world px — just
/// over half a cell so it sits over the top edge of the target cell (the move-cost-label / FCT
/// "above the cell" treatment).
const COST_LABEL_LIFT_PX: f32 = CELL_PX * 0.55;

/// `Update` ([`PresenterSystems::Draw`](crate::PresenterSystems)): draw the fire-target
/// highlight — the SINGLE RED tile UNDER the hovered enemy + the SINGLE OPAQUE TU-cost label
/// (C2).
///
/// Reads the presenter-owned [`FireTargetHighlight`] read-seam (populated by the input crate)
/// and the [`ActiveLevel`], then maintains ONE pooled [`FireTargetTile`] sprite + ONE pooled
/// [`FireTargetLabel`] cost label:
///
/// - when the highlight holds a fireable-enemy cell ON the active storey, it moves the pooled
///   tile to [`cell_to_world_layered`] at the [`Layer::FireTarget`] band (`z` strictly UNDER the
///   actor — the tile renders below the enemy sprite), tints it [`FIRE_TARGET_TINT`] (red,
///   semi-transparent), and shows it; the pooled label is moved [`COST_LABEL_LIFT_PX`] above the
///   cell, its text rewritten to [`cost_label_text`]`(cost)`, shown OPAQUE;
/// - otherwise (empty highlight / no fireable hover, OR a target off the active storey — the
///   hard cut, C5) BOTH pooled entities are [`Visibility::Hidden`] — NEVER despawned (the
///   UI-mutate-not-respawn convention, owned by the shared [`draw_pool`] walk).
///
/// Off-[`ActiveLevel`] cells are NOT drawn — the hard cut to the active storey (the GTW-347 fog /
/// GTW-358 preview precedent). Sized to one cell and drawn on
/// [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so it composites with the battlefield, not
/// the GTW-120 UI camera.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the
/// [`FireTargetHighlight`] / [`ActiveLevel`] reads, a [`TileQuery`] for the red tile, and a
/// [`LabelQuery`] for the cost label. The two pooled-entity queries are `Without` each other's
/// marker so they are provably disjoint (no B0001 conflict). Battle-gated + in
/// [`PresenterSystems::Draw`](crate::PresenterSystems) by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin).
pub fn draw_fire_target(
    mut commands: Commands,
    highlight: Res<FireTargetHighlight>,
    active: Res<ActiveLevel>,
    mut tile: TileQuery,
    mut label: LabelQuery,
) {
    let active_level: Level = **active;
    let active_z = i32::from(*active_level);
    // The drawable target = a fireable-enemy cell ON the active storey (the hard cut).
    let drawable = highlight
        .cell()
        .filter(|cell| cell.z == active_z)
        .zip(highlight.cost());

    draw_tile(&mut commands, drawable, active_level, &mut tile);
    draw_cost_label(&mut commands, drawable, active_level, &mut label);
}

/// Draw (or hide) the SINGLE RED fire-target tile for the current highlight (C2).
///
/// When `drawable` is [`Some`]`((cell, _))` the one pooled [`FireTargetTile`] is moved to the
/// cell at the [`Layer::FireTarget`] band (under the actor) and shown; otherwise it is HIDDEN.
/// Mutated in place (lazily spawned on first need). Exactly one tile entity ever exists — the
/// singleton is the shared [`draw_pool`] walk over a 0/1-length draw list (GTW-568): [`None`]
/// hides the one pooled tile via the helper's surplus sweep.
fn draw_tile(
    commands: &mut Commands,
    drawable: Option<(CellLevel, Tu)>,
    active_level: Level,
    tile_query: &mut TileQuery,
) {
    // The 0/1-length draw list: the drawable cell's world position at the FireTarget band.
    let draws = drawable.map(|(cell, _cost)| {
        cell_to_world_layered(Cell::new(cell.x, cell.y), active_level, Layer::FireTarget)
    });
    draw_pool(
        tile_query.iter_mut(),
        draws,
        |world, (transform, _)| transform.translation = world,
        |world| spawn_tile(commands, world),
        |(_, visibility)| visibility,
    );
}

/// Draw (or hide) the SINGLE OPAQUE fire-cost label for the current highlight (C2).
///
/// When `drawable` is [`Some`]`((cell, cost))` the one pooled [`FireTargetLabel`] is moved over
/// the cell (lifted [`COST_LABEL_LIFT_PX`] ABOVE it), its text rewritten to
/// [`cost_label_text`]`(cost)`, and shown; otherwise it is HIDDEN. Mutated in place (lazily
/// spawned on first need). Exactly one label entity ever exists — the singleton is the shared
/// [`draw_pool`] walk over a 0/1-length draw list (GTW-568): [`None`] hides the one pooled
/// label via the helper's surplus sweep.
fn draw_cost_label(
    commands: &mut Commands,
    drawable: Option<(CellLevel, Tu)>,
    active_level: Level,
    label_query: &mut LabelQuery,
) {
    // Above the target cell (the move-cost-label / FCT "above the cell" treatment).
    let world_at = |cell: CellLevel| {
        let mut world =
            cell_to_world_layered(Cell::new(cell.x, cell.y), active_level, Layer::FireTarget);
        world.y += COST_LABEL_LIFT_PX;
        world
    };
    draw_pool(
        label_query.iter_mut(),
        drawable,
        |(cell, cost), (label, transform, _)| {
            // Mutate the one pooled label in place (rewrite text, move; the helper shows it).
            let label: &mut Text2d = label;
            **label = cost_label_text(cost);
            transform.translation = world_at(cell);
        },
        |(cell, cost)| spawn_cost_label(commands, cost_label_text(cost), world_at(cell)),
        |(_, _, visibility)| visibility,
    );
}

/// The fire-cost label text — the bare TU count followed by `" TU"` (the firemode / move-cost /
/// status-panel TU convention).
///
/// `pub(super)` so the sibling `fire_target::test` module can pin the label format without an app
/// harness.
pub(super) fn cost_label_text(cost: Tu) -> String {
    format!("{} TU", *cost)
}

/// Lazily spawn the ONE pooled RED fire-target tile [`Sprite`] at `world`.
///
/// A one-cell semi-transparent RED [`Sprite`] on the world render layer, shown from spawn.
/// Pooled (kept + reused / moved / hidden, never despawned), so this runs only ONCE — the first
/// time a fireable enemy is hovered.
fn spawn_tile(commands: &mut Commands, world: Vec3) {
    commands.spawn((
        FireTargetTile,
        Sprite {
            color: FIRE_TARGET_TINT,
            custom_size: Some(Vec2::splat(CELL_PX)),
            ..default()
        },
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}

/// Lazily spawn the ONE pooled fire-cost label [`Text2d`] showing `text` at `world`.
///
/// A world-space [`Text2d`] anchored bottom-centre over the cell (so it sits ABOVE the target
/// cell), in OPAQUE [`COST_LABEL_COLOR`] at [`COST_LABEL_FONT_PX`], on the world render layer.
/// Pooled (kept + reused / hidden, never despawned), so this runs only ONCE — the first time a
/// fireable enemy is hovered.
fn spawn_cost_label(commands: &mut Commands, text: String, world: Vec3) {
    commands.spawn((
        FireTargetLabel,
        Text2d::new(text),
        TextFont {
            font_size: bevy::text::FontSize::Px(COST_LABEL_FONT_PX),
            ..default()
        },
        TextColor(COST_LABEL_COLOR),
        bevy::sprite::Anchor::BOTTOM_CENTER,
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
