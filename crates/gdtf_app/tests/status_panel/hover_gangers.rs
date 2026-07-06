//! AC2 hover-inspect of gangers: stat block on hover, fog gating, faction name tint.

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

/// The single entity carrying SHARED stat-block marker `M`, scoped to the INSPECT panel (a
/// descendant of the inspect panel root) — discriminates the inspect stat block's widget from
/// the status panel's.
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

/// Replaces the squad fog so EXACTLY the given cells are currently squad-VISIBLE (GTW-378).
///
/// The fog gate now suppresses the inspect panel for a fog-hidden enemy occupant, so the
/// hover tests that inspect an ENEMY ganger must seed its cell as squad-VISIBLE (the
/// real default-situation fog leaves arbitrary spawn cells UNSEEN). The VISIBLE set is its
/// own EXPLORED superset (the accrual invariant), matching `recompute_visibility`'s output.
fn make_cells_visible(app: &mut App, cells: &[CellLevel]) {
    let visible: bevy::platform::collections::HashSet<CellLevel> = cells.iter().copied().collect();
    let explored = visible.clone();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored));
}

/// AC2 — hovering a GANGER cell shows the panel + its ganger stat block.
#[test]
fn hovering_a_ganger_shows_its_stat_block() {
    let mut app = hover_app();
    let cell = CellLevel::new(Cell::new(4, 4), Level::new(0));

    // Spawn a ganger and put it on the occupancy grid at `cell`.
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
    // GTW-378 — the inspect panel now refuses a fog-hidden enemy; make the cell squad-VISIBLE
    // so this AC2 case (a SEEN enemy shows its stat block) holds under the new fog gate.
    make_cells_visible(&mut app, &[cell]);

    hover(&mut app, Some(cell));

    // The panel root is visible and the ganger stat-block host is visible.
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
    // GTW-295 — the sub-blocks show/hide via Node.display (Flex/None), so a hidden block is
    // removed from layout and never balloons the panel.
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

    // The ganger stat block's name reads the hovered ganger (the shared stat block, scoped
    // to the inspect panel).
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

    // GTW-310 — the SHARED stat block's numeric HP/TU labels also render in the INSPECT
    // panel (one change to the shared block updates both panels). The hovered ganger has
    // Tu 5/10 and Hp 10/10.
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

/// GTW-378 (regression) — hovering a FOG-HIDDEN enemy (an enemy occupant on a cell that is
/// NOT currently squad-VISIBLE) does NOT populate the inspect panel (the info-leak); making
/// the SAME cell squad-VISIBLE then DOES show it.
///
/// Pin-discriminating: it fails if the panel populates in fog (the leak the GTW-378 fix
/// closes) AND it fails if the visible-cell positive control never shows (proving the gate is
/// not merely hiding everything). Hovers a single enemy ganger across the two fog states.
#[test]
fn fog_hidden_enemy_is_not_inspected_but_a_seen_one_is() {
    let mut app = hover_app();

    // The player faction the real default-situation setup seeded.
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

    // FOG case — the cell is NOT squad-VISIBLE (empty VISIBLE set): the panel must stay hidden.
    make_cells_visible(&mut app, &[]);
    hover(&mut app, Some(cell));
    let root = single_global::<InspectPanelRoot>(&mut app);
    assert!(root.is_some(), "the inspect panel must exist");
    assert_eq!(
        root.and_then(|root| app.world().get::<Visibility>(root)),
        Some(&Visibility::Hidden),
        "hovering a FOG-HIDDEN enemy must NOT populate the inspect panel (GTW-378 info-leak)",
    );

    // VISIBLE case (positive control) — make the SAME cell squad-VISIBLE: the panel now shows.
    make_cells_visible(&mut app, &[cell]);
    hover(&mut app, Some(cell));
    if let Some(root) = single_global::<InspectPanelRoot>(&mut app) {
        assert_ne!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "hovering a SEEN enemy still shows the inspect panel (GTW-378 positive control)",
        );
    }
    assert_eq!(
        display_of::<InspectStatBlockHost>(&mut app),
        Some(Display::Flex),
        "a squad-VISIBLE enemy lays out the ganger stat block (GTW-378 positive control)",
    );
}

/// Spawns a ganger of `faction` at `cell`, puts it on the occupancy grid, and returns it.
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

/// The `TextColor` of the inspect panel's name line, if present.
fn inspect_name_color(app: &mut App) -> Option<TextColor> {
    let name = single_inspect::<StatName>(app)?;
    app.world().get::<TextColor>(name).copied()
}

/// AC2 — the inspect panel tints the NAME line by the hovered ganger's `Faction`: an ENEMY
/// (faction != `PlayerFaction`) gets a red-ish tint, a PLAYER-faction ganger gets the normal
/// theme text color. This is the AC2 panel/name COLOR tint (distinct from the "Gang N" text
/// line). Pin-discriminating: with no tint applied, the enemy name would stay the theme color
/// and the assertion fails.
#[test]
fn hovering_a_ganger_tints_the_name_by_faction() {
    let mut app = hover_app();

    // The player faction the real default-situation setup seeded (gang 0).
    let player_res = app
        .world()
        .get_resource::<PlayerFaction>()
        .copied()
        .map(|p| **p);
    assert!(player_res.is_some(), "a live battle inserts PlayerFaction");
    // The normal (player) name color is the runtime theme's body-text color.
    let normal_res = app
        .world()
        .get_resource::<GdtfTheme>()
        .map(|t| *t.text.text_color);
    assert!(normal_res.is_some(), "a live battle has the loaded theme");
    let (Some(player), Some(normal)) = (player_res, normal_res) else {
        return;
    };

    // An ENEMY ganger (a faction the player does NOT control) → red-ish tint, NOT the theme.
    let enemy_faction = Faction::new(player.wrapping_add(1));
    let enemy_cell = CellLevel::new(Cell::new(4, 4), Level::new(0));
    let player_cell = CellLevel::new(Cell::new(6, 6), Level::new(0));
    place_ganger(&mut app, enemy_cell, enemy_faction);
    // GTW-378 — both hovered cells must be squad-VISIBLE for the panel (and its name tint) to
    // populate under the new fog gate; this test inspects a SEEN enemy + a player ganger.
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

    // A PLAYER-faction ganger → the normal theme color (no enemy tint). The cell was made
    // squad-VISIBLE above (GTW-378); a player-faction occupant is trivially visible regardless.
    place_ganger(&mut app, player_cell, Faction::new(player));
    hover(&mut app, Some(player_cell));
    if let Some(color) = inspect_name_color(&mut app) {
        assert_eq!(
            color.0, normal,
            "a player-faction ganger's name uses the normal theme color",
        );
    }
}
