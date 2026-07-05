//! AC5 panel content: name + magazine, mutate-in-place, hidden/display-none empty states.

use bevy::{prelude::*, ui::Display};
use gdtf_app::test_support::{WeaponContent, WeaponMagazineText, WeaponNameText};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{Magazine, MagazineSize, ReloadTu};

use super::harness::*;

// ---------------------------------------------------------------------------------
// AC5 — name + magazine cur/max reflect the selection; selection change MUTATES in place.
// ---------------------------------------------------------------------------------

#[test]
fn weapon_panel_shows_name_and_magazine_for_the_selected_ganger() {
    let mut app = battle_running_app();
    let magazine = Magazine::new(20, MagazineSize::new(30), ReloadTu::new(12));
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

    // The widget ids BEFORE the selection change — they must be STABLE across the change.
    let name_id_before = single_with::<WeaponNameText>(&mut app);
    let mag_id_before = single_with::<WeaponMagazineText>(&mut app);
    assert_eq!(
        line_text::<WeaponNameText>(&mut app).as_deref(),
        Some("Autogun"),
    );

    // Select a DIFFERENT armed ganger with a distinct weapon + magazine.
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Lasgun",
            Magazine::new(8, MagazineSize::new(20), ReloadTu::new(10)),
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
    // Force NO selection (the auto-select may have filled it on the empty default battle).
    app.world_mut().insert_resource(SelectedShooter::cleared());
    app.update();
    assert_eq!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "with no selection the weapon content is hidden (AC9)",
    );
}

/// GTW-295 — the empty-state hide removes the content from LAYOUT via `Display::None` (so a
/// hidden weapon block takes no space), not merely `Visibility::Hidden`. Pin-discriminating:
/// a revert to a Visibility-only hide leaves `display == Display::Flex` and fails this assert.
#[test]
fn empty_state_hides_content_with_display_none() {
    let mut app = battle_running_app();

    // Armed → the content is shown (Display::Flex).
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

    // Unarmed → the content is removed from layout (Display::None).
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
    // And the existing Visibility contract still holds (GTW-275 not regressed).
    assert_eq!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "the content is also Visibility::Hidden (GTW-275 contract preserved)",
    );
}
