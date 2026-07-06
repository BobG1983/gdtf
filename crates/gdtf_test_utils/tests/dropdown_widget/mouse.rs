//! Mouse interaction: click opens above panels, click selects + emits, outside
//! click dismisses.

use bevy::{
    prelude::*,
    ui::{ComputedNode, GlobalZIndex},
};
use gdtf_ui::{DropdownBackdrop, DropdownState};

use super::harness::*;

/// The single open `DropdownBackdrop` entity, if any.
fn backdrop_of(app: &mut App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<DropdownBackdrop>>();
    q.iter(app.world()).next()
}

/// AC2 — clicking the closed control opens a FLOATING option list that EXISTS, is LAID OUT
/// (a non-zero [`ComputedNode`]), and carries a [`GlobalZIndex`] STRICTLY ABOVE the
/// contextual panel's z (the structural anti-occlusion guarantee, bevy-traps #8).
///
/// Pin-discriminating: a popup with no `GlobalZIndex` (default 0) or one at/under the panel
/// z fails the z assert; a popup that never lays out (zero `ComputedNode`) fails the layout
/// assert.
#[test]
fn click_opens_floating_list_above_panels() {
    let mut app = harness();
    let control = spawn_test_dropdown(&mut app);

    // Precondition: closed — no popup yet.
    assert!(
        popup_of(&mut app).is_none(),
        "no popup before the control is clicked",
    );

    press(&mut app, control);
    // Settle so the freshly-spawned popup gets a computed layout.
    settle(&mut app);

    let maybe_popup = popup_of(&mut app);
    assert!(
        maybe_popup.is_some(),
        "clicking the closed control must open a floating option list",
    );
    let Some(popup) = maybe_popup else { return };

    // The control's state records the open popup.
    assert!(
        matches!(
            app.world().get::<DropdownState>(control).copied(),
            Some(DropdownState::Open { .. })
        ),
        "the control's DropdownState must be Open while the list is shown",
    );

    // The popup carries a GlobalZIndex STRICTLY ABOVE the contextual panel's z.
    let z = app.world().get::<GlobalZIndex>(popup).map(|z| z.0);
    assert!(
        z.is_some_and(|z| z > CONTEXTUAL_PANEL_Z),
        "the floating list's GlobalZIndex ({z:?}) must be STRICTLY ABOVE the contextual \
         panel z ({CONTEXTUAL_PANEL_Z}) so it is never occluded (bevy-traps #8)",
    );

    // The popup is LAID OUT — its ComputedNode has a non-zero size (real geometry, not just
    // an inserted component).
    let size = app
        .world()
        .get::<ComputedNode>(popup)
        .map(ComputedNode::size);
    assert!(
        size.is_some_and(|s| s.x > 0.0 && s.y > 0.0),
        "the floating list must be laid out with a non-zero ComputedNode size (got {size:?})",
    );

    // One option row per option, all carrying the same above-panels z (it propagates).
    let rows = option_rows(&mut app);
    assert_eq!(rows.len(), 3, "the popup shows one row per option");
}

/// AC2 — selecting an option CLOSES the list, EMITS a typed
/// [`DropdownSelectionChanged`] with the right id, AND updates the shown label on the closed
/// control (mutate-in-place).
///
/// Pin-discriminating: a wrong / missing emitted id fails the message assert; a stale shown
/// label fails the relabel assert; a still-open popup fails the close assert.
#[test]
fn selecting_an_option_emits_typed_message_and_updates_label() {
    let mut app = harness();
    let control = spawn_test_dropdown(&mut app);

    // Precondition: the control shows the first option's label.
    assert_eq!(
        shown_label(&app, control).as_deref(),
        Some("Alpha"),
        "precondition: starts showing the first option",
    );

    press(&mut app, control);
    let rows = option_rows(&mut app);
    assert_eq!(rows.len(), 3, "must have opened a 3-row list");
    let bravo_row = rows[1]; // the "Bravo" / TestChoice("bravo") row

    // Press the Bravo row.
    press(&mut app, bravo_row);

    // The list closed.
    assert!(
        popup_of(&mut app).is_none(),
        "selecting an option must close the floating list",
    );
    assert_eq!(
        app.world().get::<DropdownState>(control).copied(),
        Some(DropdownState::Closed),
        "the control returns to Closed after a selection",
    );

    // The typed message carries Bravo's id + this control.
    let msgs = selection_messages(&mut app);
    assert_eq!(msgs.len(), 1, "exactly one selection message per pick");
    assert_eq!(
        msgs[0].id(),
        &TestChoice::new("bravo"),
        "the message carries the chosen option's id",
    );
    assert_eq!(
        msgs[0].control(),
        control,
        "the message carries the control's identity",
    );

    // The closed control now SHOWS Bravo's label (mutated in place).
    assert_eq!(
        shown_label(&app, control).as_deref(),
        Some("Bravo"),
        "the shown selection updates to the chosen option's label",
    );
}

/// AC2 — clicking OUTSIDE the list (the dismiss backdrop) closes it WITHOUT changing the
/// selection and WITHOUT emitting a [`DropdownSelectionChanged`].
///
/// Pin-discriminating: a backdrop press that emitted a selection, or changed the shown label,
/// or left the popup open, each fails an assert.
#[test]
fn outside_click_dismisses_without_changing_selection() {
    let mut app = harness();
    let control = spawn_test_dropdown(&mut app);

    press(&mut app, control);
    assert!(
        popup_of(&mut app).is_some(),
        "precondition: the list is open"
    );
    // Drain the open-frame messages (none expected, but keep the reader cursor clean).
    drop(selection_messages(&mut app));

    let maybe_backdrop = backdrop_of(&mut app);
    assert!(
        maybe_backdrop.is_some(),
        "an open dropdown has a dismiss backdrop",
    );
    let Some(backdrop) = maybe_backdrop else {
        return;
    };

    // Click the backdrop (an outside click).
    press(&mut app, backdrop);

    // The list closed.
    assert!(
        popup_of(&mut app).is_none(),
        "an outside click must dismiss the floating list",
    );
    assert_eq!(
        app.world().get::<DropdownState>(control).copied(),
        Some(DropdownState::Closed),
        "the control returns to Closed after an outside-click dismiss",
    );

    // The selection is UNCHANGED — still the first option, and NO selection message.
    assert_eq!(
        shown_label(&app, control).as_deref(),
        Some("Alpha"),
        "an outside click must NOT change the shown selection",
    );
    let msgs = selection_messages(&mut app);
    assert!(
        msgs.is_empty(),
        "an outside-click dismiss must emit NO DropdownSelectionChanged (got {msgs:?})",
    );
}
