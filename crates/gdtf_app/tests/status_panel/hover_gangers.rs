use bevy::{ecs::entity::Entity, prelude::*, text::TextColor, ui::Display};
use gdtf_app::test_support::{
    InspectObjectBlock, InspectPanelRoot, InspectStatBlockHost, StatHpLabel, StatName, StatTuLabel,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{GangerName, Hp, HpMax, TuMax, Wounds, WoundsMax},
    inflicted_wound::InflictedWounds,
    prelude::{
        Cell, CellLevel, Faction, Level, LifeState, OccupancyGrid, Position, Stance, StanceKind, Tu,
    },
    visibility::SquadVisibility,
};
use gdtf_ui::theme::GdtfTheme;

use super::{harness::*, hover_harness::*};

fn single_inspect<M: Component>(app: &mut App) -> Option<Entity> {
    let found: Vec<Entity> = all_with::<M>(app)
        .into_iter()
        .filter(|&e| descends_from::<InspectPanelRoot>(app, e))
        .collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Frames the screen gets to play what a fog change logged before it draws the new fog.
const CATCH_UP_FRAMES: u8 = 64;

fn make_cells_visible(app: &mut App, cells: &[CellLevel]) {
    let visible: bevy::platform::collections::HashSet<CellLevel> = cells.iter().copied().collect();
    let explored = visible.clone();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored));

    // A ganger coming into view logs an act, and the drawn fog waits for the screen to play it.
    for _ in 0..CATCH_UP_FRAMES {
        app.update();
        if screen_lights(app, cells) {
            return;
        }
    }
}

fn screen_lights(app: &App, cells: &[CellLevel]) -> bool {
    app.world()
        .get_resource::<gdtf_battle_presenter::ShownSquadVisibility>()
        .is_some_and(|shown| {
            cells
                .iter()
                .all(|at| *shown.visibility().is_cell_visible(at))
                && shown.visibility().visible_cells().count() == cells.len()
        })
}

#[test]
fn hovering_a_ganger_shows_its_stat_block() {
    let mut app = hover_app();
    let cell = CellLevel::new(Cell::new(4, 4), Level::new(0));

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(cell),
            GangerName::new("Vex Harker".to_owned()),
            Faction::new(1),
            Stance::new(StanceKind::Standing),
            Tu::new(5),
            TuMax::new(10),
            Hp::new(10),
            HpMax::new(10),
            Wounds::new(3),
            WoundsMax::new(3),
            LifeState::Alive,
            InflictedWounds::default(),
        ))
        .id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    make_cells_visible(&mut app, &[cell]);

    hover(&mut app, Some(cell));

    let root = single_global::<InspectPanelRoot>(&mut app);
    assert!(root.is_some(), "the inspect panel must exist");
    if let Some(root) = root {
        assert_ne!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "hovering a ganger shows the inspect panel",
        );
    }
    let host = single_global::<InspectStatBlockHost>(&mut app);
    assert!(
        host.is_some(),
        "the inspect panel must carry a ganger stat block"
    );
    assert_eq!(
        display_of::<InspectStatBlockHost>(&mut app),
        Some(Display::Flex),
        "hovering a ganger shows the ganger stat block (Display::Flex)",
    );
    assert_eq!(
        display_of::<InspectObjectBlock>(&mut app),
        Some(Display::None),
        "hovering a ganger removes the object block from layout (Display::None)",
    );

    if let Some(name) = single_inspect::<StatName>(&mut app) {
        let text = app
            .world()
            .get::<Text>(name)
            .map(|t| t.as_str().to_owned())
            .unwrap_or_default();
        assert!(
            text.contains("Vex Harker"),
            "inspect stat block names the ganger: {text}"
        );
    }

    if let Some(tu_label) = single_inspect::<StatTuLabel>(&mut app) {
        let text = app
            .world()
            .get::<Text>(tu_label)
            .map(|t| t.as_str().to_owned())
            .unwrap_or_default();
        assert_eq!(text, "5/10", "inspect stat block TU label reads Tu/TuMax");
    }
    if let Some(hp_label) = single_inspect::<StatHpLabel>(&mut app) {
        let text = app
            .world()
            .get::<Text>(hp_label)
            .map(|t| t.as_str().to_owned())
            .unwrap_or_default();
        assert_eq!(text, "10/10", "inspect stat block HP label reads Hp/HpMax");
    }
}

#[test]
fn fog_hidden_enemy_is_not_inspected_but_a_seen_one_is() {
    let mut app = hover_app();

    let Some(player) = app
        .world()
        .get_resource::<PlayerFaction>()
        .copied()
        .map(|p| **p)
    else {
        return;
    };
    let enemy_faction = Faction::new(player.wrapping_add(1));
    let cell = CellLevel::new(Cell::new(4, 4), Level::new(0));
    place_ganger(&mut app, cell, enemy_faction);

    make_cells_visible(&mut app, &[]);
    hover(&mut app, Some(cell));
    let root = single_global::<InspectPanelRoot>(&mut app);
    assert!(root.is_some(), "the inspect panel must exist");
    assert_eq!(
        root.and_then(|root| app.world().get::<Visibility>(root)),
        Some(&Visibility::Hidden),
        "hovering a FOG-HIDDEN enemy must NOT populate the inspect panel",
    );

    make_cells_visible(&mut app, &[cell]);
    hover(&mut app, Some(cell));
    if let Some(root) = single_global::<InspectPanelRoot>(&mut app) {
        assert_ne!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "hovering a SEEN enemy still shows the inspect panel",
        );
    }
    assert_eq!(
        display_of::<InspectStatBlockHost>(&mut app),
        Some(Display::Flex),
        "a squad-VISIBLE enemy lays out the ganger stat block",
    );
}

fn place_ganger(app: &mut App, cell: CellLevel, faction: Faction) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(cell),
            GangerName::new("Vex Harker".to_owned()),
            faction,
            Stance::new(StanceKind::Standing),
            Tu::new(5),
            TuMax::new(10),
            Hp::new(10),
            HpMax::new(10),
            Wounds::new(3),
            WoundsMax::new(3),
            LifeState::Alive,
            InflictedWounds::default(),
        ))
        .id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

fn inspect_name_color(app: &mut App) -> Option<TextColor> {
    let name = single_inspect::<StatName>(app)?;
    app.world().get::<TextColor>(name).copied()
}

#[test]
fn hovering_a_ganger_tints_the_name_by_faction() {
    let mut app = hover_app();

    let player_res = app
        .world()
        .get_resource::<PlayerFaction>()
        .copied()
        .map(|p| **p);
    assert!(player_res.is_some(), "a live battle inserts PlayerFaction");
    let normal_res = app
        .world()
        .get_resource::<GdtfTheme>()
        .map(|t| *t.text.text_color);
    assert!(normal_res.is_some(), "a live battle has the loaded theme");
    let (Some(player), Some(normal)) = (player_res, normal_res) else {
        return;
    };

    let enemy_faction = Faction::new(player.wrapping_add(1));
    let enemy_cell = CellLevel::new(Cell::new(4, 4), Level::new(0));
    let player_cell = CellLevel::new(Cell::new(6, 6), Level::new(0));
    place_ganger(&mut app, enemy_cell, enemy_faction);
    make_cells_visible(&mut app, &[enemy_cell, player_cell]);
    hover(&mut app, Some(enemy_cell));
    let enemy_color = inspect_name_color(&mut app);
    assert!(
        enemy_color.is_some(),
        "the inspect stat block carries a name color"
    );
    if let Some(color) = enemy_color {
        assert_ne!(
            color.0, normal,
            "an enemy ganger's name is tinted (NOT the normal theme color)",
        );
    }

    place_ganger(&mut app, player_cell, Faction::new(player));
    hover(&mut app, Some(player_cell));
    if let Some(color) = inspect_name_color(&mut app) {
        assert_eq!(
            color.0, normal,
            "a player-faction ganger's name uses the normal theme color",
        );
    }
}
