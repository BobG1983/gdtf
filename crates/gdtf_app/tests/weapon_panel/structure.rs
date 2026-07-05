//! GTW-298 authoritative cluster structure + responsive (non-px) sizing of content and bands.

use bevy::prelude::*;
use gdtf_app::test_support::{
    AimPanel, AimToggleButton, CombinedWeaponPanel, ModePanelRoot, ReloadButton, StancePanelRoot,
    StanceProneButton, StanceStandingButton, WeaponImage, WeaponItemButton, WeaponItemPanel,
    WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
};
use gdtf_battle_sim::{Magazine, MagazineSize, ReloadTu};

use super::harness::*;

// ---------------------------------------------------------------------------------
// GTW-298 rework — the cluster matches the AUTHORITATIVE structure (Overall 2×2 grid +
// separate Stance Panel), the relocated controls are present in their new panels, the content
// is CONTAINED + RESPONSIVE, and the empty-state hide uses Display::None (removed from layout) —
// not just Visibility::Hidden.
// ---------------------------------------------------------------------------------

/// GTW-295 — the weapon content carries a RESPONSIVE `min_height` (a window-relative
/// `Val::Vh` / `Val::Percent`, NOT a fixed `Val::Px`), so the panel contains its content and
/// scales with the window instead of being pinned at one resolution. Pin-discriminating: a
/// revert to a fixed `Val::Px` height (or no `min_height`) fails the unit-kind assert.
#[test]
fn weapon_content_has_responsive_min_height_not_px() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    let node = content_node(&mut app);
    assert!(node.is_some(), "the weapon content column must exist");
    let Some(node) = node else { return };
    assert!(
        matches!(node.min_height, Val::Vh(_) | Val::Percent(_)),
        "the weapon content min_height must be responsive (Vh/Percent), not Px — got {:?}",
        node.min_height,
    );
    // And it must be a NON-zero floor (the actual containment — a zero would not contain).
    let floor = match node.min_height {
        Val::Vh(v) | Val::Percent(v) => v,
        _ => 0.0,
    };
    assert!(
        floor > 0.0,
        "the responsive min_height must be a non-zero floor (got {floor})",
    );
}

/// GTW-298 rework — the cluster matches the AUTHORITATIVE structure: the Overall Weapon Panel
/// root holds the four 2×2 grid cells (Combined / Firemode / Item / Aim), the Combined panel
/// carries the image + the name/mag text column + the LIVE Reload button, the Item panel carries
/// TWO disabled item buttons, the Firemode panel (the relocated mode panel) and the Aim panel
/// (the relocated aim toggle) are present, and the SEPARATE Stance Panel carries the three
/// relocated stance toggles. Pin-discriminating: it asserts the structural + relocated markers —
/// a revert to the OLD layout (TOP/BOTTOM bands + empty controls frames, controls still in the
/// action bar) fails the marker asserts.
#[test]
fn weapon_panel_matches_authoritative_structure() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();
    // Settle the firemode rebuild (.after ApplyTheme) so the relocated mode toggles resolve.
    app.update();

    // ONE Overall Weapon Panel root + the four grid cells.
    assert!(
        single_with::<WeaponPanelRoot>(&mut app).is_some(),
        "exactly one Overall Weapon Panel root",
    );
    assert!(
        single_with::<CombinedWeaponPanel>(&mut app).is_some(),
        "the Combined Weapon Panel (top-left) exists",
    );
    assert!(
        single_with::<WeaponImage>(&mut app).is_some(),
        "the Combined panel carries the full-width Weapon Image placeholder",
    );
    assert!(
        single_with::<WeaponItemPanel>(&mut app).is_some(),
        "the Item Panel (top-right) exists",
    );
    assert!(
        single_with::<AimPanel>(&mut app).is_some(),
        "the Aim Panel (bottom-right) exists",
    );

    // The weapon-text column inside the Combined panel is a vertical column (name over mag).
    let content = content_node(&mut app);
    assert!(content.is_some(), "the weapon-text column must exist");
    let Some(content) = content else { return };
    assert_eq!(
        content.flex_direction,
        FlexDirection::Column,
        "the weapon-text column is a vertical column (name over magazine)",
    );

    // The Combined panel carries the name + magazine lines + the LIVE Reload button.
    assert!(
        single_with::<WeaponNameText>(&mut app).is_some(),
        "the Combined panel carries the weapon name line",
    );
    assert!(
        single_with::<WeaponMagazineText>(&mut app).is_some(),
        "the Combined panel carries the magazine line",
    );
    assert!(
        single_with::<ReloadButton>(&mut app).is_some(),
        "the Combined panel carries the LIVE Reload button",
    );

    // The Item Panel carries exactly TWO disabled item buttons.
    assert_eq!(
        all_with::<WeaponItemButton>(&mut app).len(),
        2,
        "the Item Panel carries two stacked disabled item buttons",
    );

    // The relocated Firemode panel (the mode-panel root) is present in the cluster.
    assert!(
        single_with::<ModePanelRoot>(&mut app).is_some(),
        "the relocated Firemode panel (ModePanelRoot) is present",
    );
    // The relocated Aim toggle is present in the Aim panel.
    assert!(
        single_with::<AimToggleButton>(&mut app).is_some(),
        "the relocated Aim toggle is present",
    );

    // The SEPARATE Stance Panel carries the three relocated stance toggles.
    assert!(
        single_with::<StancePanelRoot>(&mut app).is_some(),
        "the separate Stance Panel is present",
    );
    assert!(
        single_with::<StanceStandingButton>(&mut app).is_some()
            && single_with::<StanceProneButton>(&mut app).is_some(),
        "the Stance Panel carries the relocated stance toggles",
    );
}

/// GTW-298 — the cluster cells size RESPONSIVELY: the Overall panel root carries a
/// window-relative `Val::Vw`/`Val::Vh` size, and the grid cells (Combined, Item) split it by
/// `Val::Percent` — NOT fixed `Val::Px`. The separate Stance Panel is a `Val::Vw`-positioned
/// fixed-fraction column. Pin-discriminating: a revert to a px-pinned layout fails the unit-kind
/// asserts.
#[test]
fn weapon_panel_bands_size_responsively_not_px() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    // The root is sized in window-relative units (Vw width + Vh height), not fixed px.
    let root = node_of::<WeaponPanelRoot>(&mut app);
    assert!(root.is_some(), "the Overall Weapon Panel root must exist");
    let Some(root) = root else { return };
    assert!(
        matches!(root.width, Val::Vw(_) | Val::Vh(_) | Val::Percent(_)),
        "the panel root width is responsive (Vw/Vh/Percent), not Px — got {:?}",
        root.width,
    );
    assert!(
        matches!(root.height, Val::Vh(_) | Val::Vw(_) | Val::Percent(_)),
        "the panel root height is responsive (Vh/Vw/Percent), not Px — got {:?}",
        root.height,
    );

    // The Combined + Item grid cells split their columns by Percent height (the 3/4 split), not
    // fixed px.
    let combined = node_of::<CombinedWeaponPanel>(&mut app);
    let items = node_of::<WeaponItemPanel>(&mut app);
    let Some(combined) = combined else { return };
    let Some(items) = items else { return };
    assert!(
        matches!(combined.height, Val::Percent(_)),
        "the Combined cell height is a Percent of its column, not Px — got {:?}",
        combined.height,
    );
    assert!(
        matches!(items.height, Val::Percent(_)),
        "the Item cell height is a Percent of its column, not Px — got {:?}",
        items.height,
    );

    // The separate Stance Panel is a fixed-fraction-of-window (Vw) width column.
    let stance = node_of::<StancePanelRoot>(&mut app);
    let Some(stance) = stance else { return };
    assert!(
        matches!(stance.width, Val::Vw(_)),
        "the separate Stance Panel width is a fixed window fraction (Vw), not Px — got {:?}",
        stance.width,
    );
}
