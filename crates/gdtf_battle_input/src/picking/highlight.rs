//! The hover-highlight emitter (GTW-251): EMITS a presenter-owned `HighlightRequest`
//! reflecting the picked [`HoveredCell`] (the presenter LISTENS and draws the reticle).

use bevy::prelude::*;
use gdtf_battle_presenter::HighlightRequest;

use crate::picking::hovered::HoveredCell;

/// EMITS a presenter-owned [`HighlightRequest`] reflecting the picked [`HoveredCell`].
///
/// The INPUT half of the GTW-251 message-driven hover-highlight: instead of DRAWING the
/// reticle (the presenter's `draw_highlight_on_request`), it writes one
/// [`HighlightRequest`]`(`[`*hovered`](HoveredCell)`)` every update it runs —
/// [`Some(cell)`](Some) when a cell is hovered, [`None`] when nothing is. The presenter
/// LISTENS for that message and moves/shows/hides the one reticle sprite, so the drawn
/// highlight ALWAYS matches [`HoveredCell`].
///
/// Emitting EVERY update (not only on `Changed<HoveredCell>`) keeps the highlight in
/// lockstep: a per-frame re-emit of the unchanged cell is a no-op move, and a once-only emit
/// could miss the first draw if the picker resolved the cell before the presenter's reader was
/// ready.
///
/// Param-only (`bevy-traps.md` #7): a [`Res<HoveredCell>`](HoveredCell) read + a
/// [`MessageWriter<HighlightRequest>`](bevy::ecs::message::MessageWriter) write, no
/// `&mut World`. Runs `.after(pick_hovered_cell)` so it emits the SAME update's resolved
/// [`HoveredCell`], under the same `BattleInProgress` gate so it is inert pre-battle.
pub fn emit_highlight_request(
    hovered: Res<HoveredCell>,
    mut requests: MessageWriter<HighlightRequest>,
) {
    requests.write(HighlightRequest(**hovered));
}
