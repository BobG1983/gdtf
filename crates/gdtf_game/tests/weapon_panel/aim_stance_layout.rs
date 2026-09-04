use bevy::prelude::*;
use gdtf_battle_sim::{
    magazine::{Magazine, ReloadTu},
    weapon::MagazineSize,
};
use gdtf_game::test_support::{
    AimLabel, AimPanel, AimToggleButton, BottomBarRoot, StancePanelRoot, WeaponPanelRoot,
};
use gdtf_ui::themed::{ThemeRole, Themed};

use super::harness::*;

#[test]
fn aim_panel_has_aim_caption_beside_the_switch() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();
    app.update();

    assert!(
        single_with::<AimLabel>(&mut app).is_some(),
        "the Aim Panel carries the restored \"Aim\" caption (it was dropped)",
    );
    assert_eq!(
        line_text::<AimLabel>(&mut app).as_deref(),
        Some("Aim"),
        "the Aim caption reads \"Aim\"",
    );

    let cell_entity = single_with::<AimPanel>(&mut app);
    let label = single_with::<AimLabel>(&mut app);
    let switch = single_with::<AimToggleButton>(&mut app);
    assert!(
        cell_entity.is_some() && label.is_some() && switch.is_some(),
        "the Aim Panel cell, the caption, and the switch all exist",
    );
    let (Some(aim_panel), Some(label), Some(switch)) = (cell_entity, label, switch) else {
        return;
    };
    assert!(
        is_descendant_of(&app, label, aim_panel),
        "the \"Aim\" caption is laid out inside the Aim Panel cell",
    );
    assert!(
        is_descendant_of(&app, switch, aim_panel),
        "the Aim switch is laid out inside the Aim Panel cell (same cell as the caption)",
    );

    let cell = node_of::<AimPanel>(&mut app);
    assert!(cell.is_some(), "the Aim Panel cell has a Node");
    let Some(cell) = cell else { return };
    assert_eq!(
        cell.flex_direction,
        FlexDirection::Row,
        "the Aim Panel cell is a ROW so the caption sits to the LEFT of the switch",
    );
}

#[test]
fn stance_panel_is_child_of_the_bottom_bar() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    let stance = single_with::<StancePanelRoot>(&mut app);
    let bar = single_with::<BottomBarRoot>(&mut app);
    assert!(
        stance.is_some() && bar.is_some(),
        "both the Stance Panel and the bottom bar must exist in BattleRunning",
    );
    let (Some(stance), Some(bar)) = (stance, bar) else {
        return;
    };
    assert!(
        is_descendant_of(&app, stance, bar),
        "the Stance Panel must be laid out INSIDE the bottom bar (a descendant of BottomBarRoot), \
         not a free-floating overlay",
    );
}

#[test]
fn stance_panel_is_a_themed_panel_box() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    let Some(stance) = single_with::<StancePanelRoot>(&mut app) else {
        return;
    };
    assert_eq!(
        app.world().get::<Themed>(stance).map(|t| **t),
        Some(ThemeRole::Panel),
        "the Stance Panel must be its OWN bordered Themed(Panel) box (D-B), like the weapon cluster",
    );
}

#[test]
fn stance_panel_matches_overall_weapon_panel_height_relative_units() {
    let mut app = battle_running_app();
    app.update();

    let stance = node_of::<StancePanelRoot>(&mut app);
    let overall = node_of::<WeaponPanelRoot>(&mut app);
    let (Some(stance), Some(overall)) = (stance, overall) else {
        return;
    };

    assert!(
        matches!(stance.width, Val::Vw(_)),
        "the Stance Panel width is a fixed window fraction (Vw), not Px — got {:?}",
        stance.width,
    );
    assert!(
        matches!(stance.height, Val::Vh(_)),
        "the Stance Panel height is responsive (Vh), not Px — got {:?}",
        stance.height,
    );
    assert_eq!(
        stance.height, overall.height,
        "the Stance Panel height EQUALS the Overall Weapon Panel height (same height, not the full \
         bottom-bar height) — D-B",
    );
    assert_eq!(
        stance.bottom, overall.bottom,
        "the Stance Panel is anchored the SAME relative bottom as the Overall Weapon Panel (so the \
         two bordered boxes are flush-bottomed and both inset off the window edge) — got {:?} vs {:?}",
        stance.bottom, overall.bottom,
    );
    let stance_bottom = match stance.bottom {
        Val::Vh(v) => v,
        _ => 0.0,
    };
    assert!(
        matches!(stance.bottom, Val::Vh(_)) && stance_bottom > 0.0,
        "the Stance Panel is anchored a bottom-padding ABOVE the window bottom — a non-zero \
         relative Vh inset, not flush at bottom: 0 — got {:?}",
        stance.bottom,
    );
}
