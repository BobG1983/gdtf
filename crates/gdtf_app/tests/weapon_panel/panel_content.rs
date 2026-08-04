use bevy::{prelude::*, ui::Display};
use gdtf_app::test_support::{WeaponContent, WeaponMagazineText, WeaponNameText};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    magazine::{LoadedRounds, Magazine, ReloadTu},
    weapon::MagazineSize,
};

use super::harness::*;

#[test]
fn weapon_panel_shows_name_and_magazine_for_the_selected_ganger() {
    let mut app = battle_running_app();
    let magazine = Magazine::new(
        LoadedRounds::new(20),
        MagazineSize::new(30),
        ReloadTu::new(12),
    );
    spawn_armed_and_select(&mut app, weapon_kit("Autogun", magazine));
    app.update();

    let name = line_text::<WeaponNameText>(&mut app).unwrap_or_default();
    assert_eq!(
        name, "Autogun",
        "the panel shows the selected weapon's name"
    );

    let mag = line_text::<WeaponMagazineText>(&mut app).unwrap_or_default();
    assert_eq!(mag, "20/30", "the panel shows the magazine cur/max");
}

#[test]
fn selection_change_mutates_the_panel_in_place() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    let name_id_before = single_with::<WeaponNameText>(&mut app);
    let mag_id_before = single_with::<WeaponMagazineText>(&mut app);
    assert_eq!(
        line_text::<WeaponNameText>(&mut app).as_deref(),
        Some("Autogun"),
    );

    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Lasgun",
            Magazine::new(
                LoadedRounds::new(8),
                MagazineSize::new(20),
                ReloadTu::new(10),
            ),
        ),
    );
    app.update();

    let name_id_after = single_with::<WeaponNameText>(&mut app);
    let mag_id_after = single_with::<WeaponMagazineText>(&mut app);
    assert_eq!(
        name_id_before, name_id_after,
        "the name widget id is STABLE across selection change (mutate, not respawn)",
    );
    assert_eq!(
        mag_id_before, mag_id_after,
        "the magazine widget id is STABLE across selection change (mutate, not respawn)",
    );
    assert_eq!(
        line_text::<WeaponNameText>(&mut app).as_deref(),
        Some("Lasgun"),
        "the name text mutated to the new selection's weapon",
    );
    assert_eq!(
        line_text::<WeaponMagazineText>(&mut app).as_deref(),
        Some("8/20"),
        "the magazine text mutated to the new selection's cur/max",
    );
}

#[test]
fn no_selection_hides_the_weapon_content() {
    let mut app = battle_running_app();
    app.world_mut().insert_resource(SelectedShooter::cleared());
    app.update();
    assert_eq!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "with no selection the weapon content is hidden (AC9)",
    );
}

#[test]
fn empty_state_hides_content_with_display_none() {
    let mut app = battle_running_app();

    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();
    if let Some(node) = content_node(&mut app) {
        assert_eq!(
            node.display,
            Display::Flex,
            "an armed selection shows the content (Display::Flex)",
        );
    }

    spawn_unarmed_and_select(&mut app);
    app.update();
    let node = content_node(&mut app);
    assert!(
        node.is_some(),
        "the content column persists (mutate-in-place)"
    );
    let Some(node) = node else { return };
    assert_eq!(
        node.display,
        Display::None,
        "an unarmed selection removes the content from layout (Display::None)",
    );
    assert_eq!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "the content is also Visibility::Hidden (contract preserved)",
    );
}
