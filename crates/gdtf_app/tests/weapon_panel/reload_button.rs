use bevy::{prelude::*, ui::widget::Button};
use gdtf_app::test_support::{ReloadButton, WeaponContent, WeaponPanelRoot};
use gdtf_battle_input::dispatch_act_intents;
use gdtf_battle_sim::{
    acts::ReloadRequested,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    weapon::MagazineSize,
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::harness::*;

#[test]
fn reload_button_visible_with_a_magazine_hidden_without_a_weapon() {
    let mut app = battle_running_app();

    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();
    assert_ne!(
        visibility::<ReloadButton>(&mut app),
        Some(Visibility::Hidden),
        "the LIVE Reload button is shown when the weapon has a magazine (size > 0)",
    );
    assert_ne!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "the weapon content is shown for an armed selection",
    );

    spawn_unarmed_and_select(&mut app);
    app.update();
    assert_eq!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "an unarmed selection hides the weapon content (AC9 empty state)",
    );
    assert_eq!(
        visibility::<ReloadButton>(&mut app),
        Some(Visibility::Hidden),
        "no weapon → the Reload button is hidden",
    );
}

#[test]
fn pressing_reload_emits_a_reload_requested_for_the_selection() {
    let mut app = battle_running_app();
    app.init_resource::<MessageProbe<ReloadRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ReloadRequested>.after(dispatch_act_intents),
    );

    let ganger = spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::new(
                LoadedRounds::new(0),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
        ),
    );
    app.update();

    let reload_opt = single_with::<ReloadButton>(&mut app);
    assert!(
        reload_opt.is_some(),
        "the weapon panel must carry exactly one LIVE Reload button",
    );
    let Some(reload) = reload_opt else {
        return;
    };
    assert!(
        app.world().get::<Button>(reload).is_some(),
        "the Reload control is a real Button (interactive, not a label)",
    );
    press_ui_button(&mut app, reload);
    app.update();

    let reloads = probed::<ReloadRequested>(&app);
    assert_eq!(
        reloads.len(),
        1,
        "pressing Reload must emit exactly one ReloadRequested through the dispatch path",
    );
    assert_eq!(
        reloads[0].actor, ganger,
        "the ReloadRequested actor is the SelectedShooter",
    );
}

#[test]
fn weapon_panel_exists_and_is_the_only_reload_button() {
    let mut app = battle_running_app();

    assert!(
        single_with::<WeaponPanelRoot>(&mut app).is_some(),
        "the weapon panel is spawned in BattleRunning",
    );

    let reloads = all_with::<ReloadButton>(&mut app);
    assert_eq!(
        reloads.len(),
        1,
        "exactly one Reload button exists (the weapon panel's LIVE button)",
    );
}
