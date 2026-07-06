//! The member row's structure inside the scroll area: placement + exactly one
//! control per field.

use bevy::{
    app::App,
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, Children},
    },
    prelude::With,
};
use gdtf_app::test_support::{
    MemberArmorDropdown, MemberNameField, MemberRow, MemberRowIndex, MemberWeaponDropdown,
};
use gdtf_ui::ScrollListArea;

use super::harness::*;

/// How many entities carry BOTH marker `M` and a [`MemberRowIndex`] equal to `index` — the count
/// of THAT field's controls in row `index` (GTW-499 C1: must be exactly one per field).
fn count_controls_for_row<M: Component>(app: &mut App, index: usize) -> usize {
    let mut q = app.world_mut().query_filtered::<&MemberRowIndex, With<M>>();
    q.iter(app.world())
        .filter(|row_index| ***row_index == index)
        .count()
}

/// The number of DIRECT children of row `index`'s collapsed HEADER node — the `+` pip, the portrait
/// placeholder, the inline name field, the weapon dropdown, the armor dropdown, and the delete
/// button: SIX controls after the GTW-499 C1 fix.
///
/// The header carries no marker of its own, so it is located as the common parent of the row's
/// [`MemberNameField`] (a direct child the row builder `add_children`s onto the header). This is the
/// structural pin for C1: the OLD doubled layout `add_children`d THREE extra static echo `Text`
/// nodes (one beside each editable control), so its header held NINE children — re-introducing any
/// echo pushes this count above six and fails the assert.
fn header_child_count(app: &mut App, index: usize) -> usize {
    let Some(field) = control_for_row::<MemberNameField>(app, index) else {
        return 0;
    };
    let Some(header) = app.world().get::<ChildOf>(field).map(ChildOf::parent) else {
        return 0;
    };
    app.world()
        .get::<Children>(header)
        .map(|children| children.len())
        .unwrap_or_default()
}

/// Whether `descendant` has `ancestor` somewhere up its `ChildOf` chain.
fn is_ancestor(app: &App, ancestor: Entity, mut descendant: Entity) -> bool {
    while let Some(child_of) = app.world().get::<ChildOf>(descendant) {
        let parent = child_of.parent();
        if parent == ancestor {
            return true;
        }
        descendant = parent;
    }
    false
}

/// C1 + GOTCHA 1: adding a member appends a row, parented INSIDE the member-list `ScrollListArea`.
///
/// Pin: a no-op add (no new row) fails the count assert; a row parented onto the scroll-list ROOT
/// FRAME instead of the AREA (the bottom-cramp bug) fails the ancestor assert.
#[test]
fn add_member_appears_inside_scroll_area() {
    let mut app = editor_app();
    let before = count_with::<MemberRow>(&mut app);

    press_add_member(&mut app);

    let after = count_with::<MemberRow>(&mut app);
    assert_eq!(
        after,
        before + 1,
        "Add member must spawn exactly one new collapsed member row (C1)",
    );

    // The new row must descend from a ScrollListArea (the GTW-421/422 parenting rule — GOTCHA 1).
    let areas = all_with::<ScrollListArea>(&mut app);
    let rows = all_with::<MemberRow>(&mut app);
    let row = rows.first().copied().unwrap_or(Entity::PLACEHOLDER);
    assert!(
        areas.iter().any(|&area| is_ancestor(&app, area, row)),
        "a member row must be parented INSIDE a ScrollListArea (not the grid root frame) so it \
         top-anchors and scrolls (C1 / GOTCHA 1)",
    );
}

/// GTW-499 C1: a member row carries EXACTLY ONE control per field — the redundant doubled echo
/// labels are gone. For row 0 the editor must render exactly one name control ([`MemberNameField`]),
/// one weapon control ([`MemberWeaponDropdown`]), and one armor control ([`MemberArmorDropdown`]),
/// AND no extra static echo node beside any of them.
///
/// Pin-discriminating against the actual defect — the structural [`header_child_count`] assert: the
/// OLD doubled layout `add_children`d a static echo `Text` node BESIDE each editable control (a
/// SEPARATE entity carrying its OWN echo marker, not the editable marker), so its header held NINE
/// children; the fixed layout holds exactly SIX (pip, portrait, name field, weapon dropdown, armor
/// dropdown, delete). Re-introducing any of the three echo nodes (reverting the C1 fix) pushes the
/// header child count back above six and fails this assert — whereas the per-field control counts
/// alone stayed at 1 in BOTH layouts (the echo carried a distinct marker), so they cannot catch the
/// regression on their own.
#[test]
fn member_row_has_exactly_one_control_per_field() {
    let mut app = editor_app();
    press_add_member(&mut app);

    // One editable control per field (documents the per-field intent — but NOT the discriminating
    // assert: the old echo carried a distinct marker, so these counts were 1 in the doubled layout
    // too).
    assert_eq!(
        count_controls_for_row::<MemberNameField>(&mut app, 0),
        1,
        "row 0 must have exactly ONE name control (no redundant echo label — GTW-499 C1)",
    );
    assert_eq!(
        count_controls_for_row::<MemberWeaponDropdown>(&mut app, 0),
        1,
        "row 0 must have exactly ONE weapon control (no redundant echo label — GTW-499 C1)",
    );
    assert_eq!(
        count_controls_for_row::<MemberArmorDropdown>(&mut app, 0),
        1,
        "row 0 must have exactly ONE armor control (no redundant echo label — GTW-499 C1)",
    );

    // The discriminating pin: the header holds exactly the six controls — NOT the nine of the old
    // doubled layout (six + three static echo `Text` nodes). Re-adding any echo fails this.
    assert_eq!(
        header_child_count(&mut app, 0),
        6,
        "row 0's header must hold EXACTLY the six controls (pip, portrait, name field, weapon \
         dropdown, armor dropdown, delete) — the three redundant echo labels are gone; the old \
         doubled layout held nine, so re-adding any echo fails this (GTW-499 C1)",
    );
}
