//! Keyboard interaction: focus-on-open, Enter selects, Escape dismisses.

use bevy::{input_focus::InputFocus, prelude::*};
use gdtf_ui::{DropdownItem, DropdownState, focus_nav::FocusActivated};

use super::harness::*;

/// AC3 — opening the dropdown FOCUSES the first option via the existing focus helper, so the
/// existing arrow-key navigation pipeline can move the highlight (the keyboard path is wired,
/// not reinvented).
///
/// Pin-discriminating: if open did not focus an option, `InputFocus` would not point at a
/// `DropdownItem` row and the assert fails.
#[test]
fn opening_focuses_the_first_option() {
    let mut app = harness();
    let control = spawn_test_dropdown(&mut app);

    press(&mut app, control);

    let focused = app.world().resource::<InputFocus>().get();
    assert!(
        focused.is_some_and(|e| app.world().get::<DropdownItem<TestChoice>>(e).is_some()),
        "opening the dropdown must focus an option row (InputFocus -> a DropdownItem), so the \
         existing arrow-key navigation can move the highlight (AC3)",
    );
}

/// Forces `key` into the just-pressed state on `ButtonInput<KeyCode>` and runs ONLY the
/// `Update` schedule, mirroring [`press`]'s reasoning for the keyboard.
///
/// `ButtonInput` is cleared each frame in `PreUpdate` by `bevy_input`'s `InputPlugin` (which
/// the `DefaultPlugins` harness includes), so a full `app.update()` would clear the manually
/// set just-pressed edge BEFORE the `Update`-scheduled dropdown drivers read it. Running only
/// `Update` (where `dismiss_dropdowns_on_escape` + `close_dropdowns_on_dismiss_request` live)
/// preserves the edge for the real Escape chain. It runs `Update` TWICE so the
/// `DropdownDismissRequest` the first system writes is still buffered for the consumer
/// regardless of their (unordered) same-frame run order (the message survives one extra frame).
fn press_key(app: &mut App, key: KeyCode) {
    gdtf_test_utils::press_key(app, key);
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(Update);
}

/// AC3 — pressing `Enter` on a FOCUSED option row performs the FULL selection via
/// [`activate_focused_option`](gdtf_ui::activate_focused_option): it closes the popup,
/// relabels the closed control in place, and emits exactly one typed
/// [`DropdownSelectionChanged`] carrying the focused option's id + its control.
///
/// Pin-discriminating against a revert of `activate_focused_option`: it targets the index-1
/// (Bravo) row — a NON-initial option, so the `set_if_neq` change-gate genuinely fires and the
/// "Alpha" -> "Bravo" relabel is a real mutation (open focuses index 0). It drives the real
/// activation seam — set `InputFocus` to that row, then write the same-entity `FocusActivated`
/// message the keyboard bridge emits — so `activate_focused_option` reads it on the real path.
/// If the system were removed, no message is emitted, the popup stays open, and the label
/// stays "Alpha": every assert below fails.
#[test]
fn enter_on_focused_option_selects_and_emits() {
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
    let bravo_row = rows[1]; // the "Bravo" / TestChoice("bravo") row — a NON-initial option

    // Drive the real Enter-activation seam: focus the Bravo row, then write the
    // `FocusActivated` message the keyboard bridge emits for the focused entity on `Enter`.
    app.world_mut()
        .insert_resource(InputFocus::from_entity(bravo_row));
    app.world_mut()
        .write_message(FocusActivated::new(bravo_row));
    // `activate_focused_option` is `.after(FocusNavSystems::Bridge)` in `Update`; run `Update`
    // so it drains the message we wrote (bevy-traps rule 3). A second `Update` covers any
    // one-frame message-buffer latency before the assertions read the world.
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(Update);

    // The list closed and the control returned to Closed.
    assert!(
        popup_of(&mut app).is_none(),
        "Enter on a focused option must close the floating list",
    );
    assert_eq!(
        app.world().get::<DropdownState>(control).copied(),
        Some(DropdownState::Closed),
        "the control returns to Closed after a keyboard select",
    );

    // Exactly one typed message, carrying Bravo's id + this control.
    let msgs = selection_messages(&mut app);
    assert_eq!(
        msgs.len(),
        1,
        "Enter on a focused option emits exactly one selection message",
    );
    assert_eq!(
        msgs[0].id(),
        &TestChoice::new("bravo"),
        "the message carries the focused option's id",
    );
    assert_eq!(
        msgs[0].control(),
        control,
        "the message carries the control's identity",
    );

    // The closed control now SHOWS Bravo's label (mutated "Alpha" -> "Bravo" in place).
    assert_eq!(
        shown_label(&app, control).as_deref(),
        Some("Bravo"),
        "a keyboard select mutates the shown label to the chosen option",
    );
}

/// AC3 — pressing `Escape` while a dropdown is open dismisses it WITHOUT changing the
/// selection: the entire Escape chain
/// ([`dismiss_dropdowns_on_escape`](gdtf_ui::dismiss_dropdowns_on_escape) emits a
/// `DropdownDismissRequest`, and
/// [`close_dropdowns_on_dismiss_request`](gdtf_ui::close_dropdowns_on_dismiss_request) closes
/// every open dropdown) runs from a REAL key press.
///
/// Pin-discriminating against a revert of EITHER dismiss system: it drives the real
/// `Escape` KEY (via [`press_key`], which preserves the just-pressed edge for the `Update`
/// drivers), so removing the emitter leaves no request and removing the consumer leaves the
/// popup open — either way the close assert fails. The selection stays "Alpha" and NO
/// `DropdownSelectionChanged` is emitted, so an Escape that wrongly behaved like a selection
/// also fails.
#[test]
fn escape_dismisses_open_dropdown_without_changing_selection() {
    let mut app = harness();
    let control = spawn_test_dropdown(&mut app);

    press(&mut app, control);
    assert!(
        popup_of(&mut app).is_some(),
        "precondition: the list is open",
    );
    // Drain the open-frame messages (none expected) to keep the reader cursor clean.
    drop(selection_messages(&mut app));

    // Drive the real Escape key through the whole dismiss chain.
    press_key(&mut app, KeyCode::Escape);

    // The list closed and the control returned to Closed.
    assert!(
        popup_of(&mut app).is_none(),
        "pressing Escape must dismiss the open floating list",
    );
    assert_eq!(
        app.world().get::<DropdownState>(control).copied(),
        Some(DropdownState::Closed),
        "the control returns to Closed after an Escape dismiss",
    );

    // The selection is UNCHANGED — still the first option, and NO selection message.
    assert_eq!(
        shown_label(&app, control).as_deref(),
        Some("Alpha"),
        "an Escape dismiss must NOT change the shown selection",
    );
    let msgs = selection_messages(&mut app);
    assert!(
        msgs.is_empty(),
        "an Escape dismiss must emit NO DropdownSelectionChanged (got {msgs:?})",
    );
}
