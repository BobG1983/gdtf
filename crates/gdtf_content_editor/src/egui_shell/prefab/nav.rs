use bevy::prelude::*;
use gdtf_battle_presenter::ViewMode;

use crate::{
    canvas::{CurrentEditLevel, LevelStep},
    mode::EditorMode,
    session::MapEditorSession,
};

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
        let next = view.toggled();
        view.set_if_neq(next);
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};

    use crate::canvas::{CurrentEditLevel, LevelStep};

            fn size() -> GridSize {
        GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
            .unwrap_or_else(|_| GridSize::default())
    }

                #[test]
    fn level_nav_clamps_to_the_storey_range() {
        let size = size(); 
        let ground = CurrentEditLevel::ground();
        assert_eq!(*ground.level(), 0, "the editor opens on the ground storey");

        let below = ground.stepped(LevelStep::down(), size);
        assert_eq!(
            *below.level(),
            0,
            "step-down at L0 saturates at the ground storey"
        );

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
