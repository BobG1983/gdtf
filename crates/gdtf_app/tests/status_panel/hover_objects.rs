use bevy::{prelude::*, ui::Display};
use gdtf_app::test_support::{
    InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
    InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, Destroyed, HeightBand},
    occupancy::TerrainKind,
    prelude::{Cell, CellLevel, Level, OccupancyGrid},
};

use super::{harness::*, hover_harness::*};

fn global_line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = single_global::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

#[test]
fn hovering_an_object_shows_the_object_block() {
    let mut app = hover_app();
    let cell = CellLevel::new(Cell::new(7, 7), Level::new(0));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, TerrainKind::Cover);
    app.world_mut().resource_mut::<CoverLedger>().insert(
        cell,
        CoverEntry {
            current_hp:       CoverHp::new(4),
            max_hp:           CoverHp::new(8),
            height_band:      HeightBand::Mid,
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(3),
            destroyed:        Destroyed::new(false),
        },
    );

    hover(&mut app, Some(cell));

    let block = single_global::<InspectObjectBlock>(&mut app);
    assert!(
        block.is_some(),
        "the inspect panel must carry an object block"
    );
    assert_eq!(
        display_of::<InspectObjectBlock>(&mut app),
        Some(Display::Flex),
        "hovering cover shows the object block (Display::Flex)",
    );
    assert_eq!(
        display_of::<InspectStatBlockHost>(&mut app),
        Some(Display::None),
        "hovering an object removes the ganger stat block from layout (Display::None)",
    );

    let bar = single_global::<InspectObjectBar>(&mut app);
    assert!(
        bar.is_some(),
        "the object block must carry an integrity bar"
    );
    if let Some(bar) = bar {
        let integrity = bar_fill_at(&app, bar).unwrap_or(-1.0);
        assert!(
            (integrity - 50.0).abs() < 0.5,
            "the object integrity bar reads 4/8 == 50% (got {integrity})"
        );
    }

    assert_eq!(
        global_line_text::<InspectObjectText>(&mut app).as_deref(),
        Some("Cover"),
        "the object block titles the kind",
    );
    let hardness = global_line_text::<InspectObjectHardness>(&mut app).unwrap_or_default();
    assert!(
        hardness.contains('3'),
        "the Hardness line reads the seeded armor_hardness (3): {hardness}",
    );
    let protection = global_line_text::<InspectObjectProtection>(&mut app).unwrap_or_default();
    assert!(
        protection.contains('2'),
        "the Protection line reads the seeded armor_protection (2): {protection}",
    );
    let height = global_line_text::<InspectObjectHeight>(&mut app).unwrap_or_default();
    assert!(
        height.contains("Mid"),
        "the Height-band line reads the seeded height_band (Mid): {height}",
    );
}

#[test]
fn hovering_bare_floor_hides_the_panel() {
    let mut app = hover_app();
    let cell = CellLevel::new(Cell::new(20, 20), Level::new(0));
    hover(&mut app, Some(cell));

    let root = single_global::<InspectPanelRoot>(&mut app);
    assert!(root.is_some(), "the inspect panel must exist");
    if let Some(root) = root {
        assert_eq!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "hovering bare floor hides the whole inspect panel",
        );
    }
    hover(&mut app, None);
    if let Some(root) = single_global::<InspectPanelRoot>(&mut app) {
        assert_eq!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "nothing hovered hides the panel",
        );
    }
}
