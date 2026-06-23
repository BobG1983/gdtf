//! The hover-highlight emitter (GTW-251): EMITS a presenter-owned `HighlightRequest`
//! reflecting the picked LIVE hovered cell ([`InspectTarget::hovered`]) — the reticle follows
//! the cursor REGARDLESS of any pin (a pin only changes what the PANEL shows). The presenter
//! LISTENS and draws the reticle.

use bevy::prelude::*;
use gdtf_battle_presenter::{CellVisibility, HighlightRequest, cell_squad_visible};
use gdtf_battle_sim::{Faction, FactionRelation, OccupancyGrid, PlayerFaction, SquadVisibility};

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
/// GTW-11: the request now ALSO carries the cell's squad-visible
/// [`CellVisibility`](gdtf_battle_presenter::CellVisibility) verdict, computed via the SHARED
/// [`cell_squad_visible`] predicate (the SAME read the targeting hint and the
/// [`decide_left_click`](crate::selection::decide_left_click) fire-refusal consume, so reticle,
/// hint, and act never disagree). The presenter recolours the reticle off it (warm for VISIBLE,
/// cold-grey "unseen — hold your fire" for any non-VISIBLE cell). Both the mouse and the gamepad
/// flow through THIS one emitter (the picker fills [`InspectTarget`] from whichever pointer is
/// active), so the verdict is carried identically for both surfaces; there is no separate
/// gamepad emit site.
///
/// Param-only (`bevy-traps.md` #7): a [`Res<InspectTarget>`](InspectTarget) read (the LIVE
/// hovered cell, NOT the effective/pinned target — the reticle tracks the cursor under a pin),
/// `Option<`[`Res<OccupancyGrid>`](OccupancyGrid)`>` / `Option<`[`Res<SquadVisibility>`](SquadVisibility)`>`
/// / `Option<`[`Res<PlayerFaction>`](PlayerFaction)`>` reads (all fail-closed when absent), a
/// read-only `Query<&`[`Faction`]`>` for the occupant relation, and a
/// [`MessageWriter<HighlightRequest>`](bevy::ecs::message::MessageWriter) write, no
/// `&mut World`. Runs `.after(pick_hovered_cell)` so it emits the SAME update's resolved
/// hovered cell, under the same `BattleInProgress` gate so it is inert pre-battle.
pub fn emit_highlight_request(
    target: Res<InspectTarget>,
    grid: Option<Res<OccupancyGrid>>,
    squad: Option<Res<SquadVisibility>>,
    player: Option<Res<PlayerFaction>>,
    factions: Query<&Faction>,
    mut requests: MessageWriter<HighlightRequest>,
) {
    // GTW-268 — only a cell holding a ganger (occupant) or an object / cover (blocking)
    // highlights; bare floor, an off-grid hover, or an absent grid emit `None` (hide).
    let highlighted = target.hovered().and_then(|cell| {
        let grid = grid.as_ref()?;
        (grid.occupant(&cell).is_some() || grid.is_blocked(&cell)).then_some(cell)
    });
    // GTW-11 — the squad-visible verdict for the highlighted cell, via the SHARED predicate
    // (FAIL-CLOSED on an absent fog). The occupant's relation to the player squad routes the
    // predicate through `is_ganger_visible` (own-squad always visible, an enemy iff the cell is
    // VISIBLE); a blocking-terrain cell carries no occupant, so its relation is `None`.
    let visibility = highlighted.map_or(
        // No cell to highlight (hidden reticle): the verdict is irrelevant — carry the
        // not-visible default so a `None`-cell request never claims squad-visibility.
        CellVisibility::NotSquadVisible,
        |cell| {
            let relation = grid
                .as_ref()
                .and_then(|grid| grid.occupant(&cell))
                .map(|occupant| occupant_relation(occupant, &factions, player.as_deref()));
            cell_squad_visible(squad.as_deref(), &cell, relation)
        },
    );
    requests.write(HighlightRequest::new(highlighted, visibility));
}

/// The occupant's faction relation to the player squad — [`FactionRelation::OwnSquad`] when its
/// [`Faction`] equals the [`PlayerFaction`], else [`FactionRelation::Other`].
///
/// Fail-closed: an occupant with NO `Faction` component, or an absent `PlayerFaction`, is treated
/// as [`FactionRelation::Other`] (the enemy / not-yours case), so the reticle never claims a cell
/// is yours without positive faction truth.
fn occupant_relation(
    occupant: Entity,
    factions: &Query<&Faction>,
    player: Option<&PlayerFaction>,
) -> FactionRelation {
    let occupant_faction = factions.get(occupant).ok().copied();
    match (occupant_faction, player) {
        (Some(faction), Some(player)) if faction == **player => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    }
}
