//! The PREFAB-mode **level-navigation hotkeys** (GTW-515 C4.5) — the `]` / `[` and
//! `PageUp` / `PageDown` keys that step the [`CurrentEditLevel`] within the prefab's storey range.
//!
//! UI-agnostic (a plain `Update` system reading [`ButtonInput`], NOT egui), mirroring the kept
//! [`mode_hotkeys`](crate::mode::mode_hotkeys). It uses the SAME clamp the on-chrome nav buttons use
//! — [`CurrentEditLevel::stepped`], which saturates the result into `[0, levels-1]` — so a step at
//! either end is a no-op and the selector never points past the drawable volume.

use bevy::prelude::*;
use gdtf_battle_presenter::ViewMode;

use crate::{
    canvas::{CurrentEditLevel, LevelStep},
    mode::EditorMode,
    session::MapEditorSession,
};

/// `Update` (in `Editing`): the `]` / `PageUp` step UP a storey, `[` / `PageDown` step DOWN (GTW-515
/// C4.5). Guarded on the optional [`CurrentEditLevel`] + [`MapEditorSession`] (state-scoped —
/// bevy-traps #1); no-ops until they exist. The step routes through
/// [`CurrentEditLevel::stepped`], which clamps into `[0, levels-1]` (the kept level-nav clamp),
/// so a step at an end saturates rather than leaving the volume.
pub(crate) fn level_nav_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    edit_level: Option<ResMut<CurrentEditLevel>>,
    session: Option<Res<MapEditorSession>>,
) {
    let (Some(mut edit_level), Some(session)) = (edit_level, session) else {
        return;
    };
    let step = if keys.just_pressed(KeyCode::BracketRight) || keys.just_pressed(KeyCode::PageUp) {
        Some(LevelStep::up())
    } else if keys.just_pressed(KeyCode::BracketLeft) || keys.just_pressed(KeyCode::PageDown) {
        Some(LevelStep::down())
    } else {
        None
    };
    if let Some(step) = step {
        let next = edit_level.stepped(step, session.grid_size());
        edit_level.set_if_neq(next);
    }
}

/// `Update` (in `Editing`): the `F` key flips the prefab viewport's [`ViewMode`] (GTW-532 C3) —
/// the keyboard sibling of the RIGHT-panel view toggle button, flipping the SAME resource.
///
/// UI-agnostic (a plain `Update` system reading [`ButtonInput`], NOT egui), mirroring
/// [`level_nav_hotkeys`] and [`mode_hotkeys`](crate::mode::mode_hotkeys). Gated on the active
/// [`EditorMode`] being [`Prefab`](EditorMode::Prefab) so the key only toggles the storey view
/// while the prefab painter is up (it does not fire while authoring a TERRAIN / THEME def).
/// Guarded on the optional [`ViewMode`] + [`EditorMode`] (state-scoped — bevy-traps #1); no-ops
/// until they exist. Writes with [`set_if_neq`](DetectChangesMut::set_if_neq) so the change-detection
/// only fires on a real flip (the preview redraw keys off `ViewMode::is_changed`).
pub(crate) fn view_mode_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Option<Res<EditorMode>>,
    view: Option<ResMut<ViewMode>>,
) {
    let (Some(mode), Some(mut view)) = (mode, view) else {
        return;
    };
    if *mode != EditorMode::Prefab {
        return;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        // GTW-577 C8: the presenter-owned ViewMode::toggled — the SAME flip the toggle
        // button and the battlescape intent drain apply.
        let next = view.toggled();
        view.set_if_neq(next);
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};

    use crate::canvas::{CurrentEditLevel, LevelStep};

    /// A `4 × 4 × 3` volume (valid storeys 0, 1, 2), or a `1×1×1` fallback (fallible ctor,
    /// panic-free per the workspace lints).
    fn size() -> GridSize {
        GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
            .unwrap_or_else(|_| GridSize::default())
    }

    /// C4.13 — the level-nav clamp: stepping UP saturates at `levels - 1`, stepping DOWN saturates
    /// at `0`, so the selector never points past the drawable volume (the kept clamp the buttons +
    /// the hotkeys share).
    #[test]
    fn level_nav_clamps_to_the_storey_range() {
        let size = size(); // 3 storeys → valid indices 0..=2.
        let ground = CurrentEditLevel::ground();
        assert_eq!(*ground.level(), 0, "the editor opens on the ground storey");

        // Stepping DOWN at the ground floor saturates at 0 (never negative).
        let below = ground.stepped(LevelStep::down(), size);
        assert_eq!(
            *below.level(),
            0,
            "step-down at L0 saturates at the ground storey"
        );

        // Step up twice to the ceiling, then a third step saturates at levels-1 (= 2).
        let up1 = ground.stepped(LevelStep::up(), size);
        let up2 = up1.stepped(LevelStep::up(), size);
        assert_eq!(
            *up2.level(),
            2,
            "two steps up reach the top storey (levels-1)"
        );
        let up3 = up2.stepped(LevelStep::up(), size);
        assert_eq!(
            *up3.level(),
            2,
            "a step up at the ceiling saturates at levels-1 (C4.5)"
        );
    }

    /// C4.13 — a grid shrink re-clamps the current level into the smaller volume (the size-field
    /// change path calls `clamped`): a selector on storey 2 re-clamps to storey 0 when the grid
    /// shrinks to a single level.
    #[test]
    fn shrinking_the_grid_reclamps_the_level() {
        let three = size();
        let on_top = CurrentEditLevel::ground()
            .stepped(LevelStep::up(), three)
            .stepped(LevelStep::up(), three);
        assert_eq!(
            *on_top.level(),
            2,
            "start on the top storey of a 3-level grid"
        );

        let one_level = GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(1))
            .unwrap_or_else(|_| GridSize::default());
        let reclamped = on_top.clamped(one_level);
        assert_eq!(
            *reclamped.level(),
            0,
            "shrinking to one level re-clamps the selector to the ground storey (C4.5 / C4.6)",
        );
    }
}
