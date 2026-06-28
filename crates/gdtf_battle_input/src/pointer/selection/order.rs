//! The SHARED player-faction selection ordering + the Prev/Next cycle (GTW-458).
//!
//! The deterministic `(z = level, y, x)` cell ordering a player-faction ganger sorts by
//! is the SINGLE source of truth for BOTH the battle-start auto-select
//! ([`auto_select_first_player_ganger`](super::auto_select::auto_select_first_player_ganger))
//! and the GTW-458 Prev/Next selection cycle ([`cycle_player_selection`]) — so "next" /
//! "previous" step the SAME order the auto-select picks the first of, never a divergent
//! one (the allocation-order [`Entity`] id is NOT a stable order — see [`cell_order_key`]).

use gdtf_battle_sim::Position;

/// The total-ordering key a player-faction ganger sorts by for the deterministic selection
/// order (GTW-255 / GTW-458): `(level, y, x)` of its [`Position`] cell.
///
/// A `(i32, i32, i32)` tuple — the framework carve-out for an ordering key over the
/// already-typed [`Position`] coordinates (a sort key is plumbing, not a fresh domain scalar).
/// Ordered `(z = storey level, then y = row, then x = column)` so the comparison is a TOTAL
/// order over distinct cells, reproducible across runs for the same situation (unlike the
/// allocation-order [`Entity`](bevy::prelude::Entity) id). Shared by
/// [`auto_select_first_player_ganger`](super::auto_select::auto_select_first_player_ganger)
/// and [`cycle_player_selection`].
pub type CellOrderKey = (i32, i32, i32);

/// The `(level, y, x)` total-ordering key of a ganger's [`Position`] cell.
///
/// Reads the cell coordinates through [`Position`]'s [`Deref`](core::ops::Deref) to its
/// `CellLevel`/`IVec3` (`z` = storey level, `y` = row, `x` = column) and orders them
/// level-major so two gangers on the same storey break ties by row then column. The ONE
/// ordering both the auto-select and the Prev/Next cycle key off.
#[must_use]
pub fn cell_order_key(position: &Position) -> CellOrderKey {
    (position.z, position.y, position.x)
}

/// Which way the GTW-458 selection cycle steps through the player gang.
///
/// A named domain enum (no-bare-types: a cycle direction is a named choice, not a bare
/// `bool`) — [`Next`](Self::Next) advances toward the next ganger in `(z, y, x)` order,
/// [`Prev`](Self::Prev) toward the previous, both WRAPPING. The `Tab` key / the Next button
/// map to [`Next`](Self::Next); `Shift+Tab` / the Prev button map to [`Prev`](Self::Prev).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CycleDirection {
    /// Advance to the NEXT player ganger in `(z, y, x)` order (wrapping past the last to the
    /// first); from no selection, the FIRST.
    Next,
    /// Advance to the PREVIOUS player ganger in `(z, y, x)` order (wrapping past the first to
    /// the last); from no selection, the LAST.
    Prev,
}

/// Picks the [`Entity`](bevy::prelude::Entity) the selection cycle should move to — the pure,
/// deterministic GTW-458 cycle decision shared by the keyboard and the button surfaces.
///
/// `ordered` is the player-faction gangers ALREADY SORTED ascending by [`cell_order_key`]
/// (the caller does the `(z, y, x)` sort — enemies excluded). `current` is the live
/// [`SelectedShooter`](super::resources::SelectedShooter) entity, if any. Returns:
///
/// - **Empty player gang** → [`None`] (no-op; the caller leaves the selection untouched).
/// - **No current selection** → the FIRST element for [`CycleDirection::Next`], the LAST for
///   [`CycleDirection::Prev`] (cycling MAKES a first selection — AC: first-from-None).
/// - **Current selection present** → the WRAPPING neighbour: `Next` → `(i + 1) % n`,
///   `Prev` → `(i + n - 1) % n`. A `current` NOT found in `ordered` (e.g. an enemy was
///   force-selected, or the selection died) falls back to the first/last as if unselected —
///   total, never-panicking.
#[must_use]
pub fn cycle_player_selection(
    ordered: &[bevy::prelude::Entity],
    current: Option<bevy::prelude::Entity>,
    direction: CycleDirection,
) -> Option<bevy::prelude::Entity> {
    let count = ordered.len();
    if count == 0 {
        // Empty player gang — nothing to cycle to (the caller no-ops).
        return None;
    }
    // The index of the current selection within the ordered gang, if it is a member.
    let current_index = current.and_then(|entity| ordered.iter().position(|e| *e == entity));
    let next_index = match (current_index, direction) {
        // No selection (or the current is not a player ganger): MAKE a first selection at the
        // appropriate end — first for Next, last for Prev (AC first-from-None).
        (None, CycleDirection::Next) => 0,
        (None, CycleDirection::Prev) => count - 1,
        // A current selection: step with WRAP.
        (Some(index), CycleDirection::Next) => (index + 1) % count,
        (Some(index), CycleDirection::Prev) => (index + count - 1) % count,
    };
    ordered.get(next_index).copied()
}

#[cfg(test)]
mod test {
    use bevy::prelude::{Entity, World};

    use super::{CycleDirection, cycle_player_selection};

    /// `n` DISTINCT, valid [`Entity`] ids — spawned from ONE throwaway [`World`] so the test
    /// never hand-crafts a fallible raw id (the sim's `a_ganger` test-support idiom; the
    /// `bevy-traps.md` #7 carve-out (b) for a pure unit test) AND every id is guaranteed
    /// mutually distinct (entities from the SAME world have distinct indices, whereas two FRESH
    /// worlds both hand out index 0 — a collision trap). The dropped world leaves the id VALUES
    /// intact for the pure-helper comparison.
    fn distinct_entities(n: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..n).map(|_| world.spawn_empty().id()).collect()
    }

    /// Three DISTINCT entities standing in for the player gang ALREADY sorted by
    /// `cell_order_key` (the contract of `cycle_player_selection`'s `ordered` argument).
    fn ordered() -> [Entity; 3] {
        let ids = distinct_entities(3);
        [ids[0], ids[1], ids[2]]
    }

    /// C2 — Next steps forward through the ordered gang and WRAPS past the last to the first.
    #[test]
    fn next_advances_and_wraps() {
        let gang = ordered();
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[0]), CycleDirection::Next),
            Some(gang[1]),
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[1]), CycleDirection::Next),
            Some(gang[2]),
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[2]), CycleDirection::Next),
            Some(gang[0]),
            "Next wraps the last to the first",
        );
    }

    /// C2 — Prev steps backward through the ordered gang and WRAPS past the first to the last.
    #[test]
    fn prev_advances_and_wraps() {
        let gang = ordered();
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[0]), CycleDirection::Prev),
            Some(gang[2]),
            "Prev wraps the first to the last",
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[2]), CycleDirection::Prev),
            Some(gang[1]),
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[1]), CycleDirection::Prev),
            Some(gang[0]),
        );
    }

    /// C2 — from NO selection, Next picks the FIRST and Prev the LAST (first-from-None).
    #[test]
    fn from_none_picks_first_or_last() {
        let gang = ordered();
        assert_eq!(
            cycle_player_selection(&gang, None, CycleDirection::Next),
            Some(gang[0]),
            "Next from None -> first",
        );
        assert_eq!(
            cycle_player_selection(&gang, None, CycleDirection::Prev),
            Some(gang[2]),
            "Prev from None -> last",
        );
    }

    /// A `current` that is NOT a member of the ordered gang (e.g. a force-selected enemy or a
    /// dead selection) falls back to the first/last as if unselected — total, never-panicking.
    #[test]
    fn non_member_current_falls_back_to_end() {
        // FOUR distinct entities from one world; the first three are the gang, the fourth is a
        // valid id NOT in the gang (a force-selected enemy / a dead selection).
        let ids = distinct_entities(4);
        let gang = [ids[0], ids[1], ids[2]];
        let stranger = ids[3];
        assert_eq!(
            cycle_player_selection(&gang, Some(stranger), CycleDirection::Next),
            Some(gang[0]),
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(stranger), CycleDirection::Prev),
            Some(gang[2]),
        );
    }

    /// An EMPTY ordered gang → `None` for either direction (the caller no-ops).
    #[test]
    fn empty_gang_is_none() {
        assert_eq!(
            cycle_player_selection(&[], None, CycleDirection::Next),
            None,
        );
        assert_eq!(
            cycle_player_selection(&[], Some(Entity::PLACEHOLDER), CycleDirection::Prev),
            None,
        );
    }
}
