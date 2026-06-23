//! The route path-preview (E7 · GTW-12j): the presenter-owned read-seam
//! [`PathPreview`] resource (the previewed [`find_path`](gdtf_battle_sim::find_path)
//! route cells + the §48 total cost), its draw system ([`draw_path_preview`] — the
//! per-step route sprites + the C5 off-storey link-cell marker), and the cell-keyed
//! sprite marker.
//!
//! # The C6 read-seam (input → presenter → sim)
//!
//! The PRESENTER owns the read-seam ([`PathPreview`], the route [`CellLevel`] list +
//! the previewed [`Tu`] cost) plus this draw system; the INPUT crate (which alone may
//! read `SelectedShooter` + the new `PathPreviewTarget`, and which builds the GTW-353
//! [`PlanningView`](gdtf_battle_sim::PlanningView) from the sim's `SquadVisibility`)
//! calls [`find_path`](gdtf_battle_sim::find_path) and POPULATES this resource (clearing
//! it when there is no target, or the target is unreachable / only-through-UNSEEN).
//! Selection + target are NEVER pushed into the authoritative sim model, and the
//! dependency direction stays `input → presenter → sim` — the SAME shape as the
//! [`HighlightRequest`](crate::HighlightRequest)
//! seam (the presenter DEFINES the type; the input crate WRITES it). The route the
//! input crate computes uses the SAME [`find_path`](gdtf_battle_sim::find_path) +
//! `PlanningView` the move dispatch ([`dispatch_move`](gdtf_battle_sim::acts::dispatch_move))
//! uses, so the previewed route + its cost EXACTLY match what a commit will accept
//! (GTW-354/GTW-355 dependency) and the route NEVER enters an UNSEEN cell.
//!
//! # §48 cost identity (visibility.md "Pay-per-step TUs")
//!
//! The previewed cost is `path.total()` — the SAME §48 bit-identity number GTW-355's
//! `advance_walk` charges (the summed per-step entry costs), so the price the player sees
//! before committing equals what the walk spends by construction.
//!
//! # §53 EXPLORED-dimmer treatment (visibility.md "UX edges")
//!
//! A route step on a squad-EXPLORED-but-not-VISIBLE cell (remembered, not currently seen)
//! draws at a REDUCED alpha — the "remembered" treatment visibility.md flags as a presenter
//! concern; a VISIBLE step draws full. Because the input crate's `PlanningView` gate excludes
//! UNSEEN cells, a previewed route has NO unseen step (visibility.md "EXPLORED path steps
//! stay routable"), so the §53 "reduced-alpha unseen treatment" reconciles to EXPLORED-dimmer
//! (FLAGGED in the GTW-358 report).
//!
//! # Hard-cut to the active storey (the GTW-347 fog / GTW-357 overlay precedent)
//!
//! The draw hard-cuts to the presenter's [`ActiveLevel`](crate::ActiveLevel): a route cell
//! on the active storey is shown; an off-storey cell is NOT drawn. The off-storey
//! continuation is indicated ONLY by a MINIMAL marker at the LAST active-storey cell where
//! the route leaves the storey through a vertical link (the full cross-storey indicator is
//! GTW-359 — a SOFT dep; this slice draws a simple placeholder marker, FLAGGED in the report).
//!
//! # Mutate, never respawn (the UI-mutate-not-respawn convention)
//!
//! The system maintains a POOL of cell-keyed sprites (and ONE pooled target-cost label): it
//! reuses an existing entity for each route step it now needs (moving its [`Transform`],
//! re-tinting it, showing it) and HIDES surplus pooled entities it no longer needs — it never
//! despawn-then-respawns the set each frame (the [`present_fog`](crate::present_fog) precedent).

use bevy::{camera::visibility::RenderLayers, prelude::*, text::TextColor};
use gdtf_battle_sim::{Cell, CellLevel, Level, SquadVisibility, Tu};

use crate::{ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered};

/// The presenter-owned route path-preview read-seam — the cells of the
/// [`find_path`](gdtf_battle_sim::find_path) route from the SELECTED ganger to the target,
/// plus the previewed TU cost (C1 / C6).
///
/// A named domain value (no-bare-types: the previewed route + its cost is a domain value),
/// holding the route [`CellLevel`] list (`start..=goal` in step order, exactly
/// [`Path::cells`](gdtf_battle_sim::Path::cells)) and the previewed [`Tu`] cost (exactly
/// [`Path::total`](gdtf_battle_sim::Path::total) — the §48 bit-identity GTW-355 charges).
/// OWNED BY THE PRESENTER so the `input → presenter → sim` direction holds: the draw system
/// READS it; the input crate's `populate_path_preview` POPULATES it (the
/// [`HighlightRequest`](crate::HighlightRequest) precedent). `init_resource`-d by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin), so its [`Default`] is the empty
/// preview (no target → nothing drawn).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PathPreview {
    /// The route cells, `start..=goal` in step order — exactly
    /// [`Path::cells`](gdtf_battle_sim::Path::cells). Empty when there is no preview.
    cells: Vec<CellLevel>,
    /// The previewed total cost — exactly [`Path::total`](gdtf_battle_sim::Path::total),
    /// the §48 bit-identity sum GTW-355 charges. [`Tu::ZERO`](gdtf_battle_sim::Tu) for an
    /// empty preview.
    cost:  Tu,
}

impl PathPreview {
    /// Build a path preview from a found route's cells + total cost — the `(cells, total)`
    /// of a [`find_path`](gdtf_battle_sim::find_path) [`Path`](gdtf_battle_sim::Path).
    #[must_use]
    pub const fn new(cells: Vec<CellLevel>, cost: Tu) -> Self {
        Self { cells, cost }
    }

    /// The empty (no-route / no-target) preview — what the input populate system writes when
    /// there is no target, or the target is unreachable / only reachable through UNSEEN.
    #[must_use]
    pub const fn cleared() -> Self {
        Self {
            cells: Vec::new(),
            cost:  Tu::new(0),
        }
    }

    /// The previewed route cells, `start..=goal` in step order — read-only.
    #[must_use]
    pub fn cells(&self) -> &[CellLevel] {
        &self.cells
    }

    /// The previewed total cost — the §48 bit-identity sum
    /// ([`Path::total`](gdtf_battle_sim::Path::total)) GTW-355 charges.
    #[must_use]
    pub const fn cost(&self) -> Tu {
        self.cost
    }

    /// Whether the preview is empty (no route drawn).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

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

/// The translucent tint of a VISIBLE route-preview step.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a domain
/// quantity (the `CELL_PX`-class const carve-out, the [`HoverHighlight`](crate::HoverHighlight)
/// tint precedent). A warm amber at 50% alpha (GTW-371 C1: the route TILES are ~half
/// transparent so the terrain beneath reads through; the TU-cost TEXT stays FULLY OPAQUE — that
/// is the SEPARATE [`LABEL_COLOR`], untouched) so the previewed route reads as a "this is
/// where you'd walk" trail distinct from the cyan selection reticle.
const PREVIEW_TINT: Color = Color::srgba(1.0, 0.75, 0.2, 0.5);

/// The §53 EXPLORED (remembered, not currently visible) alpha SCALE applied to
/// [`PREVIEW_TINT`]'s alpha — the "remembered" treatment is dimmer than the VISIBLE step
/// (visibility.md "UX edges": a memory-tint on the previewed route's remembered steps).
const EXPLORED_ALPHA_SCALE: f32 = 0.45;

/// The C5 off-storey LINK marker tint — a cool cyan at moderate alpha, drawn at the LAST
/// active-storey cell where the route leaves the storey through a vertical link, so the
/// player sees the route continues off-storey.
///
/// A MINIMAL placeholder (the full cross-storey indicator is GTW-359 — a SOFT dep, FLAGGED
/// in the report): a single solid tint, distinct from the warm route trail.
const LINK_MARKER_TINT: Color = Color::srgba(0.3, 0.7, 1.0, 0.7);

/// `Update` ([`PresenterSystems::Draw`](crate::PresenterSystems)): draw the route
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
///   [`PREVIEW_TINT`] (full alpha when squad-VISIBLE, [`EXPLORED_ALPHA_SCALE`]-reduced when
///   squad-EXPLORED-but-not-VISIBLE — the §53 remembered treatment), and shows it;
/// - if the route LEAVES the active storey (a later route cell is on a DIFFERENT storey), it
///   draws ONE extra pooled sprite at the LAST active-storey cell tinted [`LINK_MARKER_TINT`]
///   — the C5 minimal off-storey-continuation marker (GTW-359 soft dep);
/// - every surplus pooled sprite is [`Visibility::Hidden`] — NEVER despawned (the
///   UI-mutate-not-respawn convention, the [`present_fog`](crate::present_fog)
///   precedent).
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
/// Battle-gated + in [`PresenterSystems::Draw`](crate::PresenterSystems) by the
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

    // Reuse the pooled step sprites in iteration order: move + re-tint + show the first
    // `draws.len()`, hide the rest (mutate, not respawn — the present_fog precedent).
    let mut pooled = steps.iter_mut();
    for draw in &draws {
        let world = cell_to_world_layered(
            Cell::new(draw.cell.x, draw.cell.y),
            active_level,
            Layer::PathPreview,
        );
        if let Some((mut transform, mut sprite, mut visibility)) = pooled.next() {
            transform.translation = world;
            sprite.color = draw.tint;
            *visibility = Visibility::Visible;
        } else {
            spawn_step(&mut commands, world, draw.tint);
        }
    }
    // Hide every surplus pooled sprite the current route no longer needs.
    for (_, _, mut visibility) in pooled {
        *visibility = Visibility::Hidden;
    }

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
/// storey) the label is HIDDEN. Exactly one label entity ever exists.
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

    let mut existing = label_query.iter_mut();
    match target {
        Some(cell) => {
            let mut world =
                cell_to_world_layered(Cell::new(cell.x, cell.y), active_level, Layer::PathPreview);
            // Above the target cell (the FCT / reticle-label "above the cell" treatment).
            world.y += LABEL_LIFT_PX;
            let text = label_text(preview.cost());
            if let Some((mut label, mut transform, mut visibility)) = existing.next() {
                // Mutate the one pooled label in place (rewrite text, move, show).
                **label = text;
                transform.translation = world;
                *visibility = Visibility::Visible;
            } else {
                spawn_target_label(commands, text, world);
            }
        }
        // No target on the active storey → hide the pooled label (never despawn).
        None => {
            if let Some((_, _, mut visibility)) = existing.next() {
                *visibility = Visibility::Hidden;
            }
        }
    }
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

/// One route-preview sprite to draw this update — the active-storey [`CellLevel`] and the
/// resolved tint (§53 VISIBLE vs EXPLORED, or the C5 link marker).
///
/// `pub(super)` so the sibling `path_preview::test` module can pin the §53 / hard-cut / C5
/// draw resolution without an app harness.
pub(super) struct StepDraw {
    /// The active-storey cell this sprite draws at.
    pub(super) cell: CellLevel,
    /// The resolved tint — [`PREVIEW_TINT`] (VISIBLE) or its [`EXPLORED_ALPHA_SCALE`]-reduced
    /// alpha (EXPLORED, §53), or [`LINK_MARKER_TINT`] for the C5 off-storey link marker.
    pub(super) tint: Color,
}

/// Resolve the route-preview sprites to draw this update from the [`PathPreview`] route, the
/// [`ActiveLevel`], and the squad fog — the pure draw-decision the system applies (C3 §53 +
/// C5 hard-cut + link marker).
///
/// Walks the route cells in step order:
///
/// - each cell ON the active storey becomes a [`StepDraw`] tinted by its §53 fog state
///   (VISIBLE → full [`PREVIEW_TINT`]; EXPLORED-not-VISIBLE → [`EXPLORED_ALPHA_SCALE`]-reduced
///   alpha — the remembered treatment);
/// - off-storey cells are NOT drawn (the hard cut, C5);
/// - if the route LEAVES the active storey (a route cell on a DIFFERENT storey follows an
///   active-storey cell), it appends ONE [`LINK_MARKER_TINT`] [`StepDraw`] at the LAST
///   active-storey cell before the departure — the C5 minimal off-storey-continuation marker.
///
/// `pub(super)` so the sibling `path_preview::test` module can pin the resolution directly.
pub(super) fn preview_draws(
    preview: &PathPreview,
    active_level: Level,
    squad: &SquadVisibility,
) -> Vec<StepDraw> {
    let active_z = i32::from(*active_level);
    let mut draws: Vec<StepDraw> = Vec::new();
    let mut leaves_storey_after: Option<CellLevel> = None;
    let mut seen_off_storey = false;

    for cell in preview.cells() {
        if cell.z == active_z {
            // An off-storey cell already appeared, so the route RE-ENTERS the active storey
            // (a vertical link both ways): clear the pending "leaves" marker — the route does
            // not terminate off-storey here, it dips and returns.
            leaves_storey_after = None;
            draws.push(StepDraw {
                cell: *cell,
                tint: step_tint(squad, cell),
            });
        } else {
            seen_off_storey = true;
            // The FIRST off-storey cell after an active-storey run marks where the route
            // leaves the storey — remember the last active-storey cell as the link point.
            if leaves_storey_after.is_none() {
                leaves_storey_after = draws.last().map(|d| d.cell);
            }
        }
    }

    // C5 — the minimal off-storey-continuation marker at the link cell (GTW-359 soft dep).
    // Only when the route genuinely leaves the active storey AND there is an active-storey
    // cell to mark (a route that starts off-storey has nothing to mark on this storey).
    if let Some(link_cell) = leaves_storey_after.filter(|_| seen_off_storey) {
        draws.push(StepDraw {
            cell: link_cell,
            tint: LINK_MARKER_TINT,
        });
    }

    draws
}

/// The §53 tint for a route step at `cell` — full [`PREVIEW_TINT`] when the cell is
/// squad-VISIBLE, the [`EXPLORED_ALPHA_SCALE`]-reduced alpha when it is EXPLORED-but-not-VISIBLE
/// (the remembered treatment).
///
/// The route never crosses an UNSEEN cell (the input crate's `PlanningView` gate excludes
/// them), so only the VISIBLE / EXPLORED branches are reachable for a real preview; an
/// (impossible) UNSEEN cell falls into the dimmer branch fail-safe.
fn step_tint(squad: &SquadVisibility, cell: &CellLevel) -> Color {
    if squad.is_cell_visible(cell) {
        PREVIEW_TINT
    } else {
        // EXPLORED (remembered) — dimmer; the §53 memory treatment.
        PREVIEW_TINT.with_alpha(PREVIEW_TINT.alpha() * EXPLORED_ALPHA_SCALE)
    }
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
