//! GTW-275 the bottom bar strip: existence, responsive height, padding, panel containment.

use bevy::prelude::*;
use gdtf_app::test_support::{BottomBarRoot, WeaponPanelRoot};

use super::harness::*;

// ---------------------------------------------------------------------------------
// GTW-275 layout overhaul (items 1 / 4 / 6 / 7) — the BOTTOM BAR is the one opaque strip that
// reduces the map, full-width + responsive height; the weapon panel sits IN it.
// ---------------------------------------------------------------------------------

/// GTW-275 layout overhaul (items 1 / 4) — exactly ONE bottom bar exists in `BattleRunning`,
/// sized FULL window WIDTH (`Val::Vw(100)`) with a RESPONSIVE height (a window-relative
/// `Val::Vh`/`Val::Percent`, NOT a fixed `Val::Px`). It is the only UI that reduces the map.
/// Pin-discriminating: a missing bar, a non-full width, or a px-pinned height fails the asserts.
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
    // It is anchored to the window bottom (absolute, bottom: 0) so the map ends at its top edge.
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

/// Screenshot review 2026-06-18 — the bottom panel insets its content (weapon cluster + stance
/// column) on ALL FOUR sides via `Node::padding`, in RELATIVE units (`Val::Vw` left/right,
/// `Val::Vh` top/bottom — NOT a fixed `Val::Px`), so the bottom row (Prone / firemode buttons /
/// Aim) is not flush against the window bottom and the cluster has breathing room on every edge,
/// matching the mockup. It survives the theme pass (`repad_bottom_bar` re-applies it after
/// `apply_theme`'s `box_node` clobbers `Node::padding`). Pin-discriminating: a zero / missing
/// padding, OR any edge reverted to a fixed `Val::Px`, fails the unit-kind + non-zero asserts.
#[test]
fn bottom_bar_content_has_four_sided_relative_padding() {
    let mut app = battle_running_app();
    app.update();

    let bar = node_of::<BottomBarRoot>(&mut app);
    assert!(bar.is_some(), "the bottom bar must have a Node");
    let Some(bar) = bar else { return };
    let pad = bar.padding;

    // Every edge is a non-zero, window-relative inset (Vw on the horizontal axis, Vh on the
    // vertical) — never a fixed Px (px does not survive a resize) and never zero (zero = flush).
    // A non-relative kind maps to 0.0, so the non-zero assert below catches both a Px revert and
    // a zero inset on any edge.
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
    // The horizontal edges track WIDTH (Vw) and the vertical edges track HEIGHT (Vh), so the
    // gutter is uniform-feeling on resize — guards against a swap to the wrong axis.
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

/// GTW-275 layout overhaul (item 6) / D5 / A FAIL (2026-06-18 screenshot review) — the weapon
/// panel sits IN the bottom bar: it is anchored a bottom-padding's worth ABOVE the window bottom
/// (`bottom: Vh(_)`, a NON-ZERO RELATIVE inset — NOT flush at `bottom: 0`, which jammed the
/// Firemode / Aim bottom row against the window edge) with a FIXED `Val::Vw` width (item 7 — the
/// map area does not shift when the contents change) and a responsive `Val::Vh` height that is
/// INSET inside the bar's CONTENT box — strictly LESS than the full bar height, so it neither
/// bleeds out the top (item 6) nor jams the bottom row against the window edge (the D5 inset off
/// both vertical edges). The `bottom` offset + both heights are `Val::Vh` (relative units), never
/// `Val::Px`. Pin-discriminating: a non-Vw width, a non-Vh height, a `bottom: 0` flush anchor, or a
/// panel as TALL as (or taller than) the full bar (bleed / no inset) fails.
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

    // The panel is a fixed-fraction-of-window width (item 7).
    assert!(
        matches!(panel.width, Val::Vw(_)),
        "the weapon panel width is a fixed window fraction (Vw), not Px/auto — got {:?}",
        panel.width,
    );
    // The panel is anchored a bottom-padding ABOVE the window bottom (A FAIL fix): a NON-ZERO
    // RELATIVE `Vh` inset, NOT flush at `bottom: 0` (which jammed the Firemode / Aim row against
    // the window's bottom edge). A revert to `bottom: Px(0.0)` (or any non-Vh / zero offset) fails.
    let bottom_inset = match panel.bottom {
        Val::Vh(v) => v,
        _ => 0.0,
    };
    assert!(
        matches!(panel.bottom, Val::Vh(_)) && bottom_inset > 0.0,
        "the weapon panel is anchored a bottom-padding ABOVE the window bottom — a non-zero \
         relative Vh inset, not flush at bottom: 0 — got {:?}",
        panel.bottom,
    );
    // Both heights are responsive (Vh), never Px (a non-Vh kind maps to 0.0 below, which the
    // non-zero + strict-less asserts then catch).
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
    // The panel height is INSET inside the bar's content box — strictly less than the full bar
    // height (D5: it insets off both vertical edges), so it neither bleeds out the top (item 6) nor
    // jams the bottom row against the window bottom.
    assert!(
        panel_h < bar_h,
        "the weapon panel height ({panel_h}vh) must be INSET inside the bar's content box — strictly \
         less than the full bar height ({bar_h}vh) — so it neither bleeds out the top nor jams the \
         bottom (D5 inset)",
    );
}
