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
//! [`ReachableOverlay`](crate::ReachableOverlay) / [`HighlightRequest`](crate::HighlightRequest)
//! seams (the presenter DEFINES the type; the input crate WRITES it). The route the
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
//! The system maintains a POOL of cell-keyed sprites: it reuses an existing entity for each
//! route step it now needs (moving its [`Transform`], re-tinting it, showing it) and HIDES
//! surplus pooled entities it no longer needs — it never despawn-then-respawns the set each
//! frame (the [`draw_reachable_overlay`](crate::draw_reachable_overlay) precedent).

use bevy::{camera::visibility::RenderLayers, prelude::*};
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
/// [`ReachableOverlay`](crate::ReachableOverlay) precedent). `init_resource`-d by the
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
/// justification the [`ReachableTint`](crate::ReachableTint) /
/// [`HoverHighlight`](crate::HoverHighlight) markers use): [`draw_path_preview`] queries
/// `With<PathStepSprite>` to find and MUTATE the pooled step sprites in place rather than
/// despawn-respawning them each frame.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PathStepSprite;

/// The translucent tint of a VISIBLE route-preview step.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a domain
/// quantity (the `CELL_PX`-class const carve-out, the [`ReachableTint`](crate::ReachableTint)
/// precedent). A warm amber at moderate alpha so the previewed route reads as a "this is
/// where you'd walk" trail distinct from the cool-green reachable wash and the cyan selection
/// reticle.
const PREVIEW_TINT: Color = Color::srgba(1.0, 0.75, 0.2, 0.55);

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
/// path-preview — one cell-keyed [`Sprite`] per route step on the active storey, plus a
/// MINIMAL marker at the active-storey cell where the route leaves the storey (C1 / C3 / C5).
///
/// Reads the presenter-owned [`PathPreview`] read-seam (populated by the input crate, C6),
/// the [`ActiveLevel`], and the sim's [`SquadVisibility`] (the §53 VISIBLE-vs-EXPLORED read,
/// the [`present_fog`](crate::present_fog) precedent), then maintains a POOL of
/// [`PathStepSprite`] sprites:
///
/// - for each route cell ON the active storey, it takes (or lazily spawns) a pooled sprite,
///   moves it to [`cell_to_world_layered`] at the [`Layer::PathPreview`] band, tints it
///   [`PREVIEW_TINT`] (full alpha when squad-VISIBLE, [`EXPLORED_ALPHA_SCALE`]-reduced when
///   squad-EXPLORED-but-not-VISIBLE — the §53 remembered treatment), and shows it;
/// - if the route LEAVES the active storey (a later route cell is on a DIFFERENT storey), it
///   draws ONE extra pooled sprite at the LAST active-storey cell tinted [`LINK_MARKER_TINT`]
///   — the C5 minimal off-storey-continuation marker (GTW-359 soft dep);
/// - every surplus pooled sprite is [`Visibility::Hidden`] — NEVER despawned (the
///   UI-mutate-not-respawn convention, the [`draw_reachable_overlay`](crate::draw_reachable_overlay)
///   precedent).
///
/// Off-[`ActiveLevel`] cells are NOT drawn — the hard cut to the active storey (C5; the
/// GTW-347 fog / GTW-357 overlay precedent). Sized to one cell and drawn on
/// [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so it composites with the battlefield,
/// not the GTW-120 UI camera.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the
/// [`PathPreview`] / [`ActiveLevel`] / [`SquadVisibility`] reads, and a
/// `Query<(&mut Transform, &mut Sprite, &mut Visibility), With<PathStepSprite>>` for the
/// in-place move + re-tint + show/hide. Battle-gated + in
/// [`PresenterSystems::Draw`](crate::PresenterSystems) by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin).
pub fn draw_path_preview(
    mut commands: Commands,
    preview: Res<PathPreview>,
    active: Res<ActiveLevel>,
    squad: Res<SquadVisibility>,
    mut steps: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<PathStepSprite>>,
) {
    let active_level: Level = **active;
    let draws = preview_draws(&preview, active_level, &squad);

    // Reuse the pooled step sprites in iteration order: move + re-tint + show the first
    // `draws.len()`, hide the rest (mutate, not respawn — the reachable-overlay precedent).
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
