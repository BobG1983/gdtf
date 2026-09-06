use bevy::prelude::*;
use gdtf_game::test_support::{BottomBarRoot, WeaponPanelRoot};

use super::harness::*;

// ---------------------------------------------------------------------------------

#[test]
fn bottom_bar_exists_full_width_responsive_height() {
    let mut app = battle_running_app();
    app.update();

    assert!(
        single_with::<BottomBarRoot>(&mut app).is_some(),
        "exactly one bottom bar is spawned in BattleRunning",
    );
    let bar = node_of::<BottomBarRoot>(&mut app);
    assert!(bar.is_some(), "the bottom bar must have a Node");
    let Some(bar) = bar else { return };
    assert_eq!(
        bar.width,
        Val::Vw(100.0),
        "the bottom bar is full window width (Vw 100) — items 1 / 4",
    );
    assert!(
        matches!(bar.height, Val::Vh(_) | Val::Percent(_)),
        "the bottom bar height is responsive (Vh/Percent), not Px — got {:?}",
        bar.height,
    );
    assert_eq!(
        bar.position_type,
        PositionType::Absolute,
        "the bottom bar is an absolute strip",
    );
    assert_eq!(
        bar.bottom,
        Val::Px(0.0),
        "the bottom bar is anchored to the window bottom",
    );
}

#[test]
fn bottom_bar_content_has_four_sided_relative_padding() {
    let mut app = battle_running_app();
    app.update();

    let bar = node_of::<BottomBarRoot>(&mut app);
    assert!(bar.is_some(), "the bottom bar must have a Node");
    let Some(bar) = bar else { return };
    let pad = bar.padding;

    for (edge, val) in [
        ("left", pad.left),
        ("right", pad.right),
        ("top", pad.top),
        ("bottom", pad.bottom),
    ] {
        let frac = match val {
            Val::Vw(v) | Val::Vh(v) | Val::Percent(v) => v,
            _ => 0.0,
        };
        assert!(
            frac > 0.0,
            "bottom-bar {edge} padding must be a non-zero RELATIVE inset (Vw/Vh/Percent), got {val:?} — so the content is not flush against the {edge} edge",
        );
    }
    assert!(
        matches!(pad.left, Val::Vw(_)) && matches!(pad.right, Val::Vw(_)),
        "the left/right padding tracks window WIDTH (Vw) — got {:?} / {:?}",
        pad.left,
        pad.right,
    );
    assert!(
        matches!(pad.top, Val::Vh(_)) && matches!(pad.bottom, Val::Vh(_)),
        "the top/bottom padding tracks window HEIGHT (Vh) — got {:?} / {:?}",
        pad.top,
        pad.bottom,
    );
}

#[test]
fn weapon_panel_sits_inside_the_bottom_bar() {
    let mut app = battle_running_app();
    app.update();

    let bar = node_of::<BottomBarRoot>(&mut app);
    let panel = node_of::<WeaponPanelRoot>(&mut app);
    assert!(bar.is_some(), "the bottom bar must exist");
    assert!(panel.is_some(), "the weapon panel must exist");
    let Some(bar) = bar else { return };
    let Some(panel) = panel else { return };

    assert!(
        matches!(panel.width, Val::Percent(_)),
        "the weapon panel width is a share of the bar's content box (Percent), not Px/auto. Got \
         {:?}",
        panel.width,
    );
    let bar_bottom_pad = match bar.padding.bottom {
        Val::Vh(v) => v,
        _ => 0.0,
    };
    assert!(
        panel.position_type == PositionType::Relative
            && panel.bottom == Val::Auto
            && bar_bottom_pad > 0.0,
        "the weapon panel is anchored a bottom-padding ABOVE the window bottom. It is a flex \
         child that takes that inset from the bar's own bottom padding ({:?}), never an absolute \
         inset of its own. Got position_type {:?}, bottom {:?}",
        bar.padding.bottom,
        panel.position_type,
        panel.bottom,
    );
    assert!(
        matches!(panel.height, Val::Vh(_)),
        "the weapon panel height is responsive (Vh), not Px — got {:?}",
        panel.height,
    );
    assert!(
        matches!(bar.height, Val::Vh(_)),
        "the bottom-bar height is responsive (Vh), not Px — got {:?}",
        bar.height,
    );
    let panel_h = match panel.height {
        Val::Vh(v) => v,
        _ => 0.0,
    };
    let bar_h = match bar.height {
        Val::Vh(v) => v,
        _ => 0.0,
    };
    assert!(
        panel_h < bar_h,
        "the weapon panel height ({panel_h}vh) must be INSET inside the bar's content box — strictly \
         less than the full bar height ({bar_h}vh) — so it neither bleeds out the top nor jams the \
         bottom (D5 inset)",
    );
}
