//! Ganger selection + the unified click control surface (GTW-225 / GTW-227 / GTW-238):
//! the [`SelectedShooter`] resource, the [`SelectionHighlight`] sprite, the ONE
//! disambiguated left-click decision ([`left_click_act`]), and the right-click
//! turn-to-face surface ([`right_click_turn_to_face`]) — all gated to the player's own
//! faction.
//!
//! # The unified left-click decision (GTW-238 — replaces the two-system race)
//!
//! Before GTW-238 a left-click ran TWO systems that only avoided colliding by ordering
//! (`fire_on_click.before(select_on_click)`), never a real precedence. GTW-238 REPLACES
//! that pair with ONE system, [`left_click_act`], that resolves a single Left-press edge
//! through a STRICT precedence chain — first match wins, early-return so EXACTLY ONE
//! action happens per Left edge (the RESOLVED epic precedence, confirmed by the user):
//!
//! 1. **FIRE** — a fire mode is selected AND the [`HoveredCell`](crate::HoveredCell)
//!    holds an ENEMY (occupant whose [`Faction`] `!=` [`PlayerFaction`]) AND the shared
//!    [`can_fire`](gdtf_battle_sim::can_fire) guard passes → push
//!    [`ActIntent::Fire`](crate::ActIntent::Fire).
//! 2. **SELECT** — the hovered cell holds one of YOUR gangers
//!    ([`Faction`] `==` [`PlayerFaction`]) → [`SelectedShooter::new`].
//! 3. **MOVE** — there is a player-faction selection AND the hovered cell is empty,
//!    in-bounds, and unblocked → push [`ActIntent::Move`](crate::ActIntent::Move).
//! 4. **CLEAR** — none of the above → [`SelectedShooter::cleared`].
//!
//! FALL-THROUGH: a fire mode over an EMPTY (or your-own / non-enemy) cell does NOT lock
//! out move — clause 1 fails (no enemy) and it falls through to clause 3 (MOVE). A Left
//! edge that FIRES emits NO [`MoveRequested`](gdtf_battle_sim::acts::MoveRequested) and
//! does NOT change [`SelectedShooter`] that edge; a Left edge that SELECTS emits no act.
//!
//! # Right-click turn-to-face (GTW-238)
//!
//! [`right_click_turn_to_face`]: on a Right press with a player-faction
//! [`SelectedShooter`], it reads the actor's [`Position`] cell and the
//! [`HoveredCell`](crate::HoveredCell), computes the target
//! [`Direction`](gdtf_battle_sim::Direction) via
//! [`Direction::from_cells`](gdtf_battle_sim::Direction::from_cells), and pushes
//! [`ActIntent::Turn`](crate::ActIntent::Turn). Hovering the actor's OWN cell (no
//! direction) is a no-op. The per-45deg-step turn TU cost lives in the SIM dispatch
//! (GTW-235), NOT here — this surface only emits the request.
//!
//! # Control gating — `PlayerFaction` throughout (GTW-238 / GTW-226)
//!
//! Both click surfaces read [`Res<PlayerFaction>`](gdtf_battle_sim::PlayerFaction) and a
//! `Query<&`[`Faction`]`>`: SELECT only your own faction; MOVE / TURN / FIRE act only on
//! a player-faction selection; FIRE targets only a non-player faction. A forced ENEMY
//! selection (e.g. test-injected) emits NOTHING on Left or Right.
//!
//! [`update_selection_highlight`] (the GTW-221 hover recipe applied to the selection) and
//! `sync_fire_mode_on_select` are REUSED unchanged — both still read the same
//! [`SelectedShooter`] the unified system writes.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_presenter::{ActiveLevel, CELL_PX, WORLD_RENDER_LAYER, cell_to_world};
use gdtf_battle_sim::{
    Cell, CellLevel, Direction, Faction, Level, OccupancyGrid, PlayerFaction, Position,
    acts::{MoveRequested, SetFacingRequested},
    tuning::CombatTuning,
};

use crate::{
    ActIntent, HoveredCell, PendingActIntent, SelectedFireMode,
    fire_surface::{ShooterFireData, try_fire_request},
};

/// The total-ordering key a player-faction ganger sorts by for the deterministic
/// auto-select (GTW-255): `(level, y, x)` of its [`Position`] cell.
///
/// A `(i32, i32, i32)` tuple — the framework carve-out for an ordering key over the
/// already-typed [`Position`] coordinates (a sort key is plumbing, not a fresh domain
/// scalar). Ordered `(z = storey level, then y = row, then x = column)` so the
/// comparison is a TOTAL order over distinct cells, reproducible across runs for the
/// same situation (unlike the allocation-order [`Entity`] id). See
/// [`auto_select_first_player_ganger`].
type CellOrderKey = (i32, i32, i32);

/// The `(level, y, x)` total-ordering key of a ganger's [`Position`] cell.
///
/// Reads the cell coordinates through [`Position`]'s [`Deref`] to its
/// [`CellLevel`]/`IVec3` (`z` = storey level, `y` = row, `x` = column) and orders
/// them level-major so two gangers on the same storey break ties by row then column —
/// the deterministic ordering [`auto_select_first_player_ganger`] picks the minimum of.
fn cell_order_key(position: &Position) -> CellOrderKey {
    (position.z, position.y, position.x)
}

/// The currently SELECTED shooter — the ganger a left-click picked, or `None`.
///
/// A named newtype over `Option<Entity>` (no-bare-types: the selection is a domain
/// value; [`Entity`] is the framework carve-out) that [`Deref`]s to its inner
/// [`Option`] so a reader matches it directly. `init_resource`-d by
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) — so its [`Default`] is
/// the empty selection (`None`). [`left_click_act`] writes it; the
/// [`update_selection_highlight`] sprite + the act surfaces read it.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectedShooter(pub Option<Entity>);

impl SelectedShooter {
    /// Build a selection holding `entity`.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(Some(entity))
    }

    /// The empty (no-ganger) selection — what the CLEAR branch of [`left_click_act`], or
    /// a [`SelectionClear`](crate::ActIntent::SelectionClear) intent, sets.
    #[must_use]
    pub const fn cleared() -> Self {
        Self(None)
    }
}

/// Marker for the single selection-highlight [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the
/// same justification the S7 `HoverHighlight` / the presenter's `WorldCamera`
/// markers use): [`update_selection_highlight`] queries `With<SelectionHighlight>`
/// to find and MOVE the one existing highlight rather than spawning a duplicate each
/// update. Distinct from the hover highlight so the two reticles coexist.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SelectionHighlight;

/// The translucent tint of the selection-highlight sprite.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a
/// domain quantity (the `CELL_PX`-class const carve-out, the S7 `HIGHLIGHT_TINT`
/// precedent). A cooler cyan at moderate alpha so the SELECTED cell reads distinctly
/// from the warm-white hover reticle when both are on the same cell.
const SELECTION_TINT: Color = Color::srgba(0.4, 0.85, 1.0, 0.5);

/// The read-only resources [`left_click_act`] consults, grouped into ONE [`SystemParam`]
/// so the system's parameter list stays under clippy's argument-count gate (the sim's
/// `BattleGridsParam` / the seam's `ActWriters` precedent).
///
/// Grouping the six cohesive `Res<…>` reads into one param keeps [`left_click_act`] at
/// five parameters; the body reads `reads.mouse` / `reads.player` etc. A transparent
/// system-param bundle of framework resources + landed newtypes — not itself a wrapped
/// domain scalar.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LeftClickReads<'w> {
    /// The mouse button state — the Left press edge gates the whole decision.
    mouse:     Res<'w, ButtonInput<MouseButton>>,
    /// The cell the cursor hovers (the click target), resolved last update.
    hovered:   Res<'w, HoveredCell>,
    /// The coarse occupancy grid — the occupant + `is_blocked` reads.
    occupancy: Res<'w, OccupancyGrid>,
    /// The selected fire mode — the FIRE branch's mode + the `can_fire` cone-mult.
    fire_mode: Res<'w, SelectedFireMode>,
    /// The combat tuning the shared `can_fire` guard reads.
    tuning:    Res<'w, CombatTuning>,
    /// The player's own faction — the friend/foe gate for every branch.
    player:    Res<'w, PlayerFaction>,
}

/// Resolves ONE Left-press edge through the FIRE → SELECT → MOVE → CLEAR precedence
/// chain (GTW-238), gated to the player's own faction.
///
/// On a `ButtonInput<MouseButton>` `just_pressed(Left)`, reads the hovered cell (the
/// cell resolved last update — see the ordering note below) and resolves EXACTLY ONE
/// action, first match wins, early-return:
///
/// 1. **FIRE** — a fire mode is selected, the hovered cell holds an ENEMY occupant
///    (a [`Faction`] `!=` [`PlayerFaction`]), the current selection is a player-faction
///    ganger, and the shared [`can_fire`](gdtf_battle_sim::can_fire) guard passes
///    ([`try_fire_request`]) → push [`ActIntent::Fire`]; nothing else changes this edge.
/// 2. **SELECT** — the hovered cell holds one of YOUR gangers
///    ([`Faction`] `==` [`PlayerFaction`]) → set [`SelectedShooter::new`]; no act emitted.
/// 3. **MOVE** — there is a player-faction selection AND the hovered cell is empty
///    (`occupant == None`), in-bounds (a [`Some`] [`HoveredCell`] is always in-grid), and
///    unblocked (`!is_blocked`) → push [`ActIntent::Move`] to the hovered destination.
/// 4. **CLEAR** — none of the above → [`SelectedShooter::cleared`].
///
/// FALL-THROUGH (the user-confirmed precedence): a fire mode over an EMPTY / your-own /
/// non-enemy cell FAILS clause 1 (no enemy occupant) and falls through to clause 3
/// (MOVE) — the fire mode does NOT lock out move. The branches are MUTUALLY EXCLUSIVE: a
/// FIRE edge emits no [`MoveRequested`] and does not touch [`SelectedShooter`]; a SELECT
/// edge emits no act message.
///
/// Param-only (`bevy-traps.md` #7): the [`LeftClickReads`] read bundle + a read-only
/// `Query<&Faction>` (occupant + selection faction) + a read-only `Query<ShooterFireData>`
/// (the FIRE branch's `can_fire` reads), the [`ResMut<SelectedShooter>`] / the
/// [`ResMut<PendingActIntent>`] writes — no `&mut World`. Runs `.before(pick_hovered_cell)`
/// (`bevy-traps.md` #3) so it reads the cell resolved last update (exact for a one-frame
/// click) and `.before(dispatch_act_intents)` so the drain sees this update's pushes.
pub fn left_click_act(
    reads: LeftClickReads,
    factions: Query<&Faction>,
    shooters: Query<ShooterFireData>,
    mut selected: ResMut<SelectedShooter>,
    mut pending: ResMut<PendingActIntent>,
) {
    // Only act on the press edge; a held button does not re-resolve.
    if !reads.mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // Nothing hovered -> no cell to act on -> CLEAR (clause 4, the no-hover case).
    let Some(target) = **reads.hovered else {
        set_selection(&mut selected, SelectedShooter::cleared());
        return;
    };
    let player = **reads.player;
    // The occupant at the hovered cell (None when empty), and its faction (None when the
    // occupant carries no `Faction` component — fail-closed, treated as not-yours).
    let occupant = reads.occupancy.occupant(&target);
    let occupant_faction = occupant.and_then(|e| factions.get(e).ok().copied());
    // Whether the CURRENT selection is a player-faction ganger (gates FIRE + MOVE).
    let selection_is_player = (**selected)
        .and_then(|e| factions.get(e).ok().copied())
        .is_some_and(|f| f == player);

    // 1. FIRE — a fire mode + an ENEMY occupant + a player-faction selection + can_fire.
    if let (Some(shooter), Some(enemy_faction)) = (**selected, occupant_faction)
        && selection_is_player
        && enemy_faction != player
        && let Some(request) =
            try_fire_request(shooter, target, &reads.fire_mode, &reads.tuning, &shooters)
    {
        pending.push(ActIntent::Fire(request));
        return; // FIRE wins this edge — no select change, no move.
    }

    // 2. SELECT — the hovered cell holds one of YOUR gangers.
    if let (Some(entity), Some(faction)) = (occupant, occupant_faction)
        && faction == player
    {
        set_selection(&mut selected, SelectedShooter::new(entity));
        return; // SELECT wins — no act emitted.
    }

    // 3. MOVE — a player-faction selection + an empty, in-bounds, unblocked hovered cell.
    if let Some(actor) = **selected
        && selection_is_player
        && occupant.is_none()
        && !reads.occupancy.is_blocked(&target)
    {
        pending.push(ActIntent::Move(MoveRequested::new(actor, target)));
        return; // MOVE wins.
    }

    // 4. CLEAR — none of the above.
    set_selection(&mut selected, SelectedShooter::cleared());
}

/// Turns the player-faction [`SelectedShooter`] to face the [`HoveredCell`] on a Right
/// press (GTW-238), pushing an [`ActIntent::Turn`].
///
/// On a `ButtonInput<MouseButton>` `just_pressed(Right)` with a player-faction
/// [`SelectedShooter`]: reads the actor's [`Position`] cell and the
/// [`HoveredCell`](crate::HoveredCell), computes the target
/// [`Direction`](gdtf_battle_sim::Direction) via
/// [`Direction::from_cells`](gdtf_battle_sim::Direction::from_cells) (actor's cell → the
/// hovered cell), and pushes [`ActIntent::Turn`]`(`[`SetFacingRequested`]`)`. The
/// per-45deg-step turn TU cost is the SIM's facing dispatch (GTW-235), NOT here — this
/// surface only emits the request.
///
/// Writes NO intent when: the button is not just-pressed; there is no selection; the
/// selection is NOT a player-faction ganger (gating, AC6); nothing is hovered; the actor
/// has no [`Position`] (fail-closed via the query lookup); or the hovered cell is the
/// actor's OWN cell (`from_cells` returns [`None`] — no direction toward yourself).
/// Param-only (`bevy-traps.md` #7): all reads via `Res` / `Query`, the intent push via
/// [`ResMut<PendingActIntent>`]; no `&mut World`. Runs `.before(pick_hovered_cell)` (the
/// cell resolved last update) and `.before(dispatch_act_intents)` (the drain).
pub fn right_click_turn_to_face(
    mouse: Res<ButtonInput<MouseButton>>,
    hovered: Res<HoveredCell>,
    player: Res<PlayerFaction>,
    selected: Res<SelectedShooter>,
    factions: Query<&Faction>,
    positions: Query<&Position>,
    mut pending: ResMut<PendingActIntent>,
) {
    // Only act on the press edge; a held button does not re-turn.
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    // No selection -> nothing to turn (AC6).
    let Some(actor) = **selected else {
        return;
    };
    // Gating: only a PLAYER-faction selection turns (a forced enemy emits nothing, AC6).
    let Ok(faction) = factions.get(actor) else {
        return;
    };
    if *faction != **player {
        return;
    }
    // Nothing hovered -> no target cell.
    let Some(target) = **hovered else {
        return;
    };
    // The actor's grid cell (fail-closed if it has no Position component).
    let Ok(position) = positions.get(actor) else {
        return;
    };
    let actor_cell = Cell::new(position.x, position.y);
    let hovered_cell = Cell::new(target.x, target.y);
    // Hovering the actor's OWN cell yields no direction -> no-op (no intent).
    let Some(facing) = Direction::from_cells(actor_cell, hovered_cell) else {
        return;
    };
    pending.push(ActIntent::Turn(SetFacingRequested::new(actor, facing)));
}

/// Writes `next` into `selected` only on a real change (change-detection hygiene), so a
/// no-op CLEAR/SELECT does not spuriously trip `Changed<SelectedShooter>` (which
/// `sync_fire_mode_on_select` keys off).
fn set_selection(selected: &mut ResMut<SelectedShooter>, next: SelectedShooter) {
    if **selected != next {
        **selected = next;
    }
}

/// Sets the INITIAL [`SelectedShooter`] to the deterministic player-faction ganger when
/// the battle becomes live with nothing selected yet (GTW-255).
///
/// Fixes the "battle opens with no unit selected" play-test bug: at setup
/// [`SelectedShooter`] is `init_resource`-d to [`None`], so the player would otherwise
/// have to hunt-and-click before any control works. This system fills that EMPTY
/// selection ONCE with one of the player's own gangers, so the highlight + fire-mode
/// default are live the moment the battle opens — exactly as for a click selection.
///
/// Behaviour:
///
/// - **No-op unless the selection is empty.** It only acts when `**selected == None`;
///   it NEVER overrides a selection the player (or any other system) already made — it
///   sets the INITIAL selection once.
/// - **Deterministic ganger.** Among the gangers whose [`Faction`] `==`
///   [`PlayerFaction`], it selects the one with the lowest [`Position`] cell by the
///   `(level, y, x)` total ordering ([`cell_order_key`]) — reproducible across runs for
///   the same situation, NOT the allocation-order [`Entity`] id. (FLAGGED alternative:
///   authored / spawn order, were a stable spawn-index component present — there is
///   none today, so the cell ordering is the stable choice.)
/// - **Never an enemy.** It reads `Res<`[`PlayerFaction`]`>` and only considers gangers
///   whose own [`Faction`] equals it; an enemy-faction ganger is never auto-selected. No
///   player-faction ganger present → the selection stays [`None`].
///
/// OUT OF SCOPE (flagged, not built this slice): re-selecting when the selected ganger
/// dies / downs or the turn advances — this sets the INITIAL selection only.
///
/// Param-only (`bevy-traps.md` #7): a read-only `Query<(`[`Entity`]`, &`[`Faction`]`,
/// &`[`Position`]`)>` + `Res<`[`PlayerFaction`]`>` + the [`ResMut<SelectedShooter>`]
/// write — no `&mut World`. Gated
/// `run_if(resource_exists::<`[`BattleInProgress`](gdtf_battle_sim::BattleInProgress)`>`
/// `.and` `resource_exists::<`[`PlayerFaction`]`>)` (`bevy-traps.md` #1), placed in
/// [`InputSystems::Gather`](crate::InputSystems::Gather) ordered
/// `.before(`[`left_click_act`]`)` alongside the other selection writers, so the same
/// update's `sync_fire_mode_on_select` (`fire_mode.rs`) +
/// [`update_selection_highlight`] react to the new selection exactly as for a click.
pub fn auto_select_first_player_ganger(
    gangers: Query<(Entity, &Faction, &Position)>,
    player: Res<PlayerFaction>,
    mut selected: ResMut<SelectedShooter>,
) {
    // Only ever FILL an empty selection — never override an existing one (the INITIAL
    // selection is set once).
    if selected.is_some() {
        return;
    }
    let player_faction = **player;
    // The deterministic player-faction ganger: the lowest `Position` cell by the
    // `(level, y, x)` total ordering. `min_by_key` returns `None` when the player has
    // no gangers, leaving the selection empty (never an enemy).
    let pick = gangers
        .iter()
        .filter(|(_, faction, _)| **faction == player_faction)
        .min_by_key(|(_, _, position)| cell_order_key(position))
        .map(|(entity, ..)| entity);
    if let Some(entity) = pick {
        set_selection(&mut selected, SelectedShooter::new(entity));
    }
}

/// Maintains exactly ONE selection-highlight sprite that snaps to [`SelectedShooter`].
///
/// The S7 [`update_hover_highlight`](crate::update_hover_highlight) recipe applied to
/// the SELECTED ganger: spawns the single [`SelectionHighlight`] sprite the first
/// time a ganger is selected; on every later update it MOVES that one sprite's
/// [`Transform`] to [`cell_to_world`] of the occupant's cell (read off the
/// [`OccupancyGrid`] occupant entry on the presenter's [`ActiveLevel`]) and shows it,
/// or HIDES it ([`Visibility::Hidden`]) when nothing is selected (or the selected
/// ganger is not on the active storey). Sized to one cell
/// (`custom_size: Some(Vec2::splat(CELL_PX))`) and drawn on
/// [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so it composites with the
/// battlefield, not the GTW-120 UI camera.
///
/// The selected ganger's CELL is found by scanning the [`OccupancyGrid`] on the
/// active level for the slot whose occupant is the selected entity — the presenter
/// holds no sim→cell map, and the occupancy grid is the authoritative
/// entity→`(cell, level)` source (a read-only peek, never mutated). No match on the
/// active level → the highlight hides (the selected ganger is on another storey).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawn, a
/// `Query<(&mut Transform, &mut Visibility), With<SelectionHighlight>>` for the move +
/// show/hide, `Res<SelectedShooter>` / `Res<OccupancyGrid>` / `Res<ActiveLevel>`
/// reads. Runs `.after(left_click_act)` so it reads the same update's selection.
pub fn update_selection_highlight(
    mut commands: Commands,
    selected: Res<SelectedShooter>,
    occupancy: Res<OccupancyGrid>,
    active_level: Res<ActiveLevel>,
    mut highlights: Query<(&mut Transform, &mut Visibility), With<SelectionHighlight>>,
) {
    // The cell the selected ganger occupies on the active level (None when nothing is
    // selected or the selected ganger is on another storey), projected to world.
    let target = (**selected)
        .and_then(|entity| selected_cell(&occupancy, **active_level, entity))
        .map(|cell| cell_to_world(cell, **active_level));

    match highlights.single_mut() {
        Ok((mut transform, mut visibility)) => match target {
            Some(world) => {
                transform.translation = world;
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        },
        // No highlight yet: spawn the single sprite the first time a ganger is
        // selected on the active level. (When nothing is selected there is nothing to
        // spawn — it stays absent until the first selection, equivalent to hidden.)
        Err(_) => {
            if let Some(world) = target {
                commands.spawn((
                    SelectionHighlight,
                    Sprite {
                        color: SELECTION_TINT,
                        custom_size: Some(Vec2::splat(CELL_PX)),
                        ..default()
                    },
                    Transform::from_translation(world),
                    // Explicitly Visible (not the `Inherited` default) so the reticle
                    // shows from the first frame, independent of parent visibility.
                    Visibility::Visible,
                    RenderLayers::layer(WORLD_RENDER_LAYER),
                ));
            }
        }
    }
}

/// The ground-plane [`Cell`] the `entity` occupies on `level`, by scanning the
/// [`OccupancyGrid`]'s slots for the one whose occupant is `entity`.
///
/// A read-only peek (never mutates the grid): the grid is the authoritative
/// entity→`(cell, level)` source and the presenter holds no sim→cell map. Returns
/// [`None`] when `entity` is not occupying any cell on `level` (e.g. it is on a
/// different storey), so the highlight hides. Bounded by the 60×60 grid extent.
fn selected_cell(occupancy: &OccupancyGrid, level: Level, entity: Entity) -> Option<Cell> {
    use gdtf_battle_sim::{GRID_HEIGHT, GRID_WIDTH};
    for y in 0..grid_extent_i32(GRID_HEIGHT) {
        for x in 0..grid_extent_i32(GRID_WIDTH) {
            let cell = Cell::new(x, y);
            if occupancy.occupant(&CellLevel::new(cell, level)) == Some(entity) {
                return Some(cell);
            }
        }
    }
    None
}

/// Widens a `usize` grid extent to `i32` for the [`selected_cell`] scan bounds.
///
/// 60 fits `i32` comfortably, but `as i32` on a `usize` trips `cast_possible_wrap`
/// (`-D`); [`i32::try_from`] is the no-`unwrap` cast — the S7 `grid_extent_i32`
/// precedent. An unrepresentable extent saturates to [`i32::MAX`] (only widens the
/// scan, never narrows it).
fn grid_extent_i32(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}
