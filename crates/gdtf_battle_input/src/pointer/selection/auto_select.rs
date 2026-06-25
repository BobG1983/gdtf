//! The battle-start auto-select (GTW-255): [`auto_select_first_player_ganger`] sets the INITIAL
//! [`SelectedShooter`] once to the deterministic player-faction ganger.

use bevy::prelude::*;
use gdtf_battle_sim::{Faction, PlayerFaction, Position};

use crate::selection::resources::{SelectedShooter, set_selection};

/// The total-ordering key a player-faction ganger sorts by for the deterministic auto-select
/// (GTW-255): `(level, y, x)` of its [`Position`] cell.
///
/// A `(i32, i32, i32)` tuple — the framework carve-out for an ordering key over the
/// already-typed [`Position`] coordinates (a sort key is plumbing, not a fresh domain scalar).
/// Ordered `(z = storey level, then y = row, then x = column)` so the comparison is a TOTAL
/// order over distinct cells, reproducible across runs for the same situation (unlike the
/// allocation-order [`Entity`] id). See [`auto_select_first_player_ganger`].
type CellOrderKey = (i32, i32, i32);

/// The `(level, y, x)` total-ordering key of a ganger's [`Position`] cell.
///
/// Reads the cell coordinates through [`Position`]'s [`Deref`] to its `CellLevel`/`IVec3`
/// (`z` = storey level, `y` = row, `x` = column) and orders them level-major so two gangers on
/// the same storey break ties by row then column.
fn cell_order_key(position: &Position) -> CellOrderKey {
    (position.z, position.y, position.x)
}

/// Sets the INITIAL [`SelectedShooter`] to the deterministic player-faction ganger when the
/// battle becomes live with nothing selected yet (GTW-255).
///
/// Fixes the "battle opens with no unit selected" play-test bug: at setup [`SelectedShooter`]
/// is `init_resource`-d to [`None`]. This system fills that EMPTY selection ONCE with one of
/// the player's own gangers, so the highlight + fire-mode default are live the moment the battle
/// opens — exactly as for a click selection.
///
/// Behaviour:
///
/// - **No-op unless the selection is empty.** It only acts when `**selected == None`; it NEVER
///   overrides a selection the player (or any other system) already made.
/// - **Deterministic ganger.** Among the gangers whose [`Faction`] `==` [`PlayerFaction`], it
///   selects the one with the lowest [`Position`] cell by the `(level, y, x)` total ordering
///   ([`cell_order_key`]) — reproducible across runs, NOT the allocation-order [`Entity`] id.
/// - **Never an enemy.** It reads `Res<`[`PlayerFaction`]`>` and only considers gangers whose
///   own [`Faction`] equals it. No player-faction ganger present → the selection stays [`None`].
///
/// OUT OF SCOPE (flagged): re-selecting when the selected ganger dies / downs or the turn
/// advances — this sets the INITIAL selection only.
///
/// Param-only (`bevy-traps.md` #7): a read-only `Query<(`[`Entity`]`, &`[`Faction`]`,
/// &`[`Position`]`)>` + `Res<`[`PlayerFaction`]`>` + the [`ResMut<SelectedShooter>`] write — no
/// `&mut World`. Gated `run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>))`
/// (`bevy-traps.md` #1), placed in [`InputSystems::Gather`](crate::InputSystems) ordered
/// `.before(`[`left_click_act`](crate::left_click_act)`)`.
pub fn auto_select_first_player_ganger(
    gangers: Query<(Entity, &Faction, &Position)>,
    player: Res<PlayerFaction>,
    mut selected: ResMut<SelectedShooter>,
) {
    // Only ever FILL an empty selection — never override an existing one.
    if selected.is_some() {
        return;
    }
    let player_faction = **player;
    // The deterministic player-faction ganger: the lowest `Position` cell by the
    // `(level, y, x)` total ordering. `min_by_key` returns `None` when the player has no
    // gangers, leaving the selection empty (never an enemy).
    let pick = gangers
        .iter()
        .filter(|(_, faction, _)| **faction == player_faction)
        .min_by_key(|(_, _, position)| cell_order_key(position))
        .map(|(entity, ..)| entity);
    if let Some(entity) = pick {
        set_selection(&mut selected, SelectedShooter::new(entity));
    }
}
