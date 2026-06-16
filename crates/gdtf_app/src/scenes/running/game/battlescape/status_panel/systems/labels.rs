//! Pure format helpers that render a selected ganger's typed vital components into
//! their display strings (GTW-252, AC5).
//!
//! Each helper takes the real typed sim component(s) and returns the line's
//! [`String`] — no [`App`](bevy::prelude::App), no `World`, no ECS. They are the
//! presentation policy of the status panel (how a vital reads on screen), isolated
//! so they are unit-testable directly and the update system stays a thin wiring
//! layer. The vocabulary (Standing / Crouching / Prone, Alive / Downed / Dead)
//! matches the sim's named domain enums.

use gdtf_battle_sim::{Faction, Hp, LifeState, Position, Stance, StanceKind, Tu, TuMax, Wounds};

/// The line shown when there is no selection — a clear empty state (AC4).
///
/// A `const` so the empty-state copy lives in one place and the update system can
/// write it without re-allocating a literal per line per frame.
pub(in crate::scenes::running::game::battlescape::status_panel) const NO_SELECTION: &str =
    "No ganger selected";

/// The **identity** line: the ganger's cell `(x, y, level)` and its faction (gang)
/// index.
///
/// There is NO ganger-name component today, so the cell + faction stand in for an
/// identity (a real `GangerName` newtype is flagged future work). [`Position`]
/// derefs to the `(cell, level)` key — `x`/`y` are the ground-plane cell, `z` is
/// the storey index. [`Faction`] derefs to its small gang index. `Position` (a
/// 12-byte `IVec3`) is taken by reference; the 1-byte [`Faction`] is taken by value
/// (clippy `trivially_copy_pass_by_ref` — every component here is `Copy`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn identity_label(
    position: &Position,
    faction: Faction,
) -> String {
    format!(
        "Cell ({}, {}, lvl {})  Gang {}",
        position.x, position.y, position.z, *faction,
    )
}

/// The **stance** line: the ganger's posture by its [`StanceKind`] name.
///
/// [`Stance`] derefs to the [`StanceKind`] posture; the rendered word matches the
/// sim's named vocabulary (resolution.md's stand / kneel / prone postures). Taken by
/// value — `Stance` is a 1-byte `Copy` newtype (clippy `trivially_copy_pass_by_ref`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn stance_label(
    stance: Stance,
) -> String {
    let word = match *stance {
        StanceKind::Standing => "Standing",
        StanceKind::Crouching => "Crouching",
        StanceKind::Prone => "Prone",
    };
    format!("Stance: {word}")
}

/// The **time-units** line: the current [`Tu`] pool over the [`TuMax`] ceiling, as
/// `cur/max` (e.g. "TU 7/10").
///
/// Both newtypes deref to their inner `u8` budget — current is the live pool, max the
/// round-start ceiling (the GTW-38 reaction-ratio denominator). Taken by value — each
/// is a 1-byte `Copy` newtype (clippy `trivially_copy_pass_by_ref`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn tu_label(
    tu: Tu,
    tu_max: TuMax,
) -> String {
    format!("TU {}/{}", *tu, *tu_max)
}

/// The **hit-points** line: the current [`Hp`] pool plus the [`Wounds`] count.
///
/// There is NO `HpMax` component today, so HP is shown as a bare current value (no
/// fabricated max — an `HpMax` source is flagged future work). [`Hp`] derefs to its
/// `u16` knock-down pool, [`Wounds`] to its `u8` life pool. Taken by value — both are
/// small `Copy` newtypes (clippy `trivially_copy_pass_by_ref`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn hp_label(
    hp: Hp,
    wounds: Wounds,
) -> String {
    format!("HP {}  Wounds {}", *hp, *wounds)
}

/// The **life-state** line: the ganger's terminal [`LifeState`] by name.
///
/// The two-pool outcome (Alive / Downed / Dead), matching the sim's named state
/// machine. Taken by value — [`LifeState`] is a 1-byte `Copy` enum (clippy
/// `trivially_copy_pass_by_ref`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn life_label(
    life: LifeState,
) -> String {
    let word = match life {
        LifeState::Alive => "Alive",
        LifeState::Downed => "Downed",
        LifeState::Dead => "Dead",
    };
    format!("State: {word}")
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        Cell, CellLevel, Faction, Hp, Level, LifeState, Position, Stance, StanceKind, Tu, TuMax,
        Wounds,
    };

    use super::{hp_label, identity_label, life_label, stance_label, tu_label};

    /// The identity line names the ganger's cell coordinates, storey, and faction —
    /// each component's value appears, so swapping x↔y or faction would be visible.
    #[test]
    fn identity_label_names_cell_and_faction() {
        let position = Position::new(CellLevel::new(Cell::new(5, 6), Level::new(2)));
        let faction = Faction::new(1);
        let label = identity_label(&position, faction);
        assert!(
            label.contains('5'),
            "identity must show the cell x: {label}"
        );
        assert!(
            label.contains('6'),
            "identity must show the cell y: {label}"
        );
        assert!(
            label.contains('2'),
            "identity must show the storey level: {label}",
        );
        assert!(
            label.contains("Gang 1"),
            "identity must show the faction index: {label}",
        );
    }

    /// Each stance posture renders its own distinct word — discriminating: a
    /// mis-wired posture would surface the wrong word here.
    #[test]
    fn stance_label_renders_each_posture() {
        assert!(stance_label(Stance::new(StanceKind::Standing)).contains("Standing"));
        assert!(stance_label(Stance::new(StanceKind::Crouching)).contains("Crouching"));
        assert!(stance_label(Stance::new(StanceKind::Prone)).contains("Prone"));
    }

    /// The TU line renders the current pool over the max as `cur/max`.
    #[test]
    fn tu_label_is_current_over_max() {
        let label = tu_label(Tu::new(7), TuMax::new(10));
        assert!(
            label.contains("7/10"),
            "TU line must read current/max: {label}",
        );
    }

    /// A spent-down TU pool renders the new current, never the max — guards against
    /// a line wired to the wrong component (max where current was meant).
    #[test]
    fn tu_label_reflects_spent_pool() {
        let full = tu_label(Tu::new(10), TuMax::new(10));
        let spent = tu_label(Tu::new(3), TuMax::new(10));
        assert!(full.contains("10/10"), "full pool: {full}");
        assert!(spent.contains("3/10"), "spent pool: {spent}");
        assert_ne!(full, spent, "spending TU must change the rendered line");
    }

    /// The HP line shows the current HP and the Wounds count — no fabricated max.
    #[test]
    fn hp_label_shows_current_hp_and_wounds() {
        let label = hp_label(Hp::new(8), Wounds::new(3));
        assert!(label.contains('8'), "HP line must show current HP: {label}");
        assert!(
            label.contains('3'),
            "HP line must show the Wounds count: {label}",
        );
    }

    /// Each life state renders its own distinct word.
    #[test]
    fn life_label_renders_each_state() {
        assert!(life_label(LifeState::Alive).contains("Alive"));
        assert!(life_label(LifeState::Downed).contains("Downed"));
        assert!(life_label(LifeState::Dead).contains("Dead"));
    }
}
