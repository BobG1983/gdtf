//! AC6/AC8/AC9 the Reload button: visibility with/without magazine, press emits `ReloadRequested`, only-reload-button.

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

// ---------------------------------------------------------------------------------
// AC6 / AC9 — Reload button visibility matches magazine presence; empty state hidden.
// ---------------------------------------------------------------------------------

#[test]
fn reload_button_visible_with_a_magazine_hidden_without_a_weapon() {
    let mut app = battle_running_app();

    // Armed with a magazine (size > 0) → the content + reload button are Visible.
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

    // Unarmed selection → the content (and so the reload button) are Hidden (AC9 empty state).
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

// ---------------------------------------------------------------------------------
// AC6 — pressing Reload emits ActIntent::Reload → ReloadRequested through the input queue.
// ---------------------------------------------------------------------------------

#[test]
fn pressing_reload_emits_a_reload_requested_for_the_selection() {
    let mut app = battle_running_app();
    // The generic GTW-576 probe, drained at the ORIGINAL observation point (Update,
    // after the intent drain) so it pins the drain's same-update emission.
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

    // Synthesize a press on the LIVE Reload button (the cursor production is covered
    // elsewhere; the panel only needs the Interaction::Pressed transition).
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

// ---------------------------------------------------------------------------------
// AC8 — the action-bar no longer carries a Reload button (the removed stub).
// ---------------------------------------------------------------------------------

#[test]
fn weapon_panel_exists_and_is_the_only_reload_button() {
    let mut app = battle_running_app();

    // The weapon panel root exists in BattleRunning.
    assert!(
        single_with::<WeaponPanelRoot>(&mut app).is_some(),
        "the weapon panel is spawned in BattleRunning",
    );

    // There is exactly ONE Reload button, and it lives under the weapon panel — the old
    // action-bar Reload stub is GONE (AC8). (The action-bar `ReloadButton` marker was
    // removed entirely, so the only `ReloadButton` is the weapon panel's.)
    let reloads = all_with::<ReloadButton>(&mut app);
    assert_eq!(
        reloads.len(),
        1,
        "exactly one Reload button exists (the weapon panel's LIVE button)",
    );
}
