//! The hover-highlight emitter (GTW-251): EMITS a presenter-owned `HighlightRequest`
//! reflecting the picked LIVE hovered cell ([`InspectTarget::hovered`]) — the reticle follows
//! the cursor REGARDLESS of any pin (a pin only changes what the PANEL shows). The presenter
//! LISTENS and draws the reticle.

use bevy::prelude::*;
use gdtf_battle_presenter::HighlightRequest;
use gdtf_battle_sim::OccupancyGrid;

use crate::picking::hovered::InspectTarget;

/// EMITS a presenter-owned [`HighlightRequest`] reflecting the picked LIVE hovered cell
/// ([`InspectTarget::hovered`]), GATED so only OCCUPIED or BLOCKING cells highlight (GTW-268 —
/// gangers + objects, never bare floor).
///
/// The INPUT half of the GTW-251 message-driven hover-highlight: instead of DRAWING the
/// reticle (the presenter's `draw_highlight_on_request`), it writes one
/// [`HighlightRequest`] every update it runs. The presenter LISTENS for that message and
/// moves/shows/hides the one reticle sprite, so the drawn highlight ALWAYS matches what
/// this emits.
///
/// GTW-268: the hovered cell is emitted as [`Some(cell)`](Some) ONLY when the
/// [`OccupancyGrid`] says the cell either has an occupant
/// ([`occupant`](OccupancyGrid::occupant)`.is_some()` — a ganger) OR is blocking
/// ([`is_blocked`](OccupancyGrid::is_blocked) — cover / a wall / an object). Hovering bare
/// floor (or no cell at all) emits [`HighlightRequest`]`(`[`None`]`)`, so the reticle hides
/// — the user's decision that only gangers and objects, never empty floor, highlight. When
/// the grid is ABSENT (e.g. a harness with no sim plugin) the gate fails closed to [`None`]
/// (nothing to highlight without occupancy truth), keeping the system panic-free
/// (`bevy-traps.md` #1).
///
/// Emitting EVERY update (not only on `Changed<InspectTarget>`) keeps the highlight in
/// lockstep: a per-frame re-emit is a no-op move, and a once-only emit could miss the first
/// draw if the picker resolved the cell before the presenter's reader was ready.
///
/// Param-only (`bevy-traps.md` #7): a [`Res<InspectTarget>`](InspectTarget) read (the LIVE
/// hovered cell, NOT the effective/pinned target — the reticle tracks the cursor under a pin),
/// an `Option<`[`Res<OccupancyGrid>`](OccupancyGrid)`>` read, and a
/// [`MessageWriter<HighlightRequest>`](bevy::ecs::message::MessageWriter) write, no
/// `&mut World`. Runs `.after(pick_hovered_cell)` so it emits the SAME update's resolved
/// hovered cell, under the same `BattleInProgress` gate so it is inert pre-battle.
pub fn emit_highlight_request(
    target: Res<InspectTarget>,
    grid: Option<Res<OccupancyGrid>>,
    mut requests: MessageWriter<HighlightRequest>,
) {
    // GTW-268 — only a cell holding a ganger (occupant) or an object / cover (blocking)
    // highlights; bare floor, an off-grid hover, or an absent grid emit `None` (hide).
    let highlighted = target.hovered().and_then(|cell| {
        let grid = grid.as_ref()?;
        (grid.occupant(&cell).is_some() || grid.is_blocked(&cell)).then_some(cell)
    });
    requests.write(HighlightRequest::new(highlighted));
}
