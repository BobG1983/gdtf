//! The pure draw-decision for the route preview: the §53 EXPLORED-dimmer tinting,
//! the hard cut to the active storey, and the C5 off-storey link marker.
//!
//! # §53 EXPLORED-dimmer treatment (visibility.md "UX edges")
//!
//! A route step on a squad-EXPLORED-but-not-VISIBLE cell (remembered, not currently seen)
//! draws at a REDUCED alpha — the "remembered" treatment visibility.md flags as a presenter
//! concern; a VISIBLE step draws full. Because the input crate's `PlanningView` gate excludes
//! UNSEEN cells, a previewed route has NO unseen step (visibility.md "EXPLORED path steps
//! stay routable"), so the §53 "reduced-alpha unseen treatment" reconciles to EXPLORED-dimmer
//! (FLAGGED in the GTW-358 report).

use bevy::prelude::*;
use gdtf_battle_sim::{
    prelude::{CellLevel, Level},
    visibility::SquadVisibility,
};

use super::seam::PathPreview;

/// The translucent tint of a VISIBLE route-preview step.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a domain
/// quantity (the `CELL_PX`-class const carve-out, the [`HoverHighlight`](crate::HoverHighlight)
/// tint precedent). A warm amber at 50% alpha (GTW-371 C1: the route TILES are ~half
/// transparent so the terrain beneath reads through; the TU-cost TEXT stays FULLY OPAQUE — that
/// is the SEPARATE [`LABEL_COLOR`](super::draw::LABEL_COLOR), untouched) so the previewed route
/// reads as a "this is
/// where you'd walk" trail distinct from the cyan selection reticle. `pub(super)` so the
/// sibling draw / test modules can reference the shared swatch.
pub(super) const PREVIEW_TINT: Color = Color::srgba(1.0, 0.75, 0.2, 0.5);

/// The §53 EXPLORED (remembered, not currently visible) alpha SCALE applied to
/// [`PREVIEW_TINT`]'s alpha — the "remembered" treatment is dimmer than the VISIBLE step
/// (visibility.md "UX edges": a memory-tint on the previewed route's remembered steps).
pub(super) const EXPLORED_ALPHA_SCALE: f32 = 0.45;

/// The C5 off-storey LINK marker tint — a cool cyan at moderate alpha, drawn at the LAST
/// active-storey cell where the route leaves the storey through a vertical link, so the
/// player sees the route continues off-storey.
///
/// A MINIMAL placeholder (the full cross-storey indicator is GTW-359 — a SOFT dep, FLAGGED
/// in the report): a single solid tint, distinct from the warm route trail.
pub(super) const LINK_MARKER_TINT: Color = Color::srgba(0.3, 0.7, 1.0, 0.7);

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
/// [`ActiveLevel`](crate::ActiveLevel), and the squad fog — the pure draw-decision the system
/// applies (C3 §53 + C5 hard-cut + link marker).
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
