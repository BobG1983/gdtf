//! GTW-410 headless integration test for the [`Dropdown`](gdtf_ui::Dropdown) widget, driving
//! the REAL widget on the REAL `bevy_ui` layout path (verification rule 3).
//!
//! These run on the [`GdtfUiTestAppBuilder`] `DefaultPlugins` headless harness (real layout
//! geometry — a computed [`ComputedNode`] — and a real camera) with [`gdtf_ui::UiPlugin`] and
//! `register_dropdown::<TestChoice>` added, so the dropdown's open / select / dismiss drivers
//! actually run. Each test is pin-discriminating: it would fail if the z-order, the selection
//! message, the shown-label mutation, or the dismiss-without-change broke.

use bevy::{
    input::ButtonInput,
    input_focus::InputFocus,
    prelude::*,
    ui::{BackgroundColor, ComputedNode, ComputedStackIndex, GlobalZIndex, Interaction},
};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use gdtf_ui::{
    DropdownBackdrop, DropdownColors, DropdownItem, DropdownItemMarker, DropdownLabel,
    DropdownOption, DropdownPopup, DropdownSelectionChanged, DropdownState, UiPlugin,
    focus_nav::FocusActivated, register_dropdown, spawn_dropdown,
};

/// The test's option identity — a named newtype over the choice name (no-bare-types rule).
#[derive(Clone, PartialEq, Eq, Debug)]
struct TestChoice(String);

impl TestChoice {
    /// Wraps a choice name.
    fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// The z-band the contextual panel renders on — the highest existing UI layer the floating
/// dropdown list must clear (mirrors `CONTEXTUAL_PANEL_Z` in `gdtf_app`).
const CONTEXTUAL_PANEL_Z: i32 = 20;

/// A distinct test color set so asserts discriminate.
const COLORS: DropdownColors = DropdownColors {
    control_bg:          Color::srgb(0.2, 0.2, 0.25),
    text:                Color::srgb(0.9, 0.9, 0.8),
    popup_bg:            Color::srgb(0.1, 0.1, 0.15),
    option_bg:           Color::srgb(0.15, 0.15, 0.2),
    option_highlight_bg: Color::srgb(0.45, 0.62, 0.30),
};

/// Builds the harness: the headless UI app + `UiPlugin` + the `TestChoice` dropdown
/// registration, so the real drivers run on a real layout.
fn harness() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin);
    register_dropdown::<TestChoice>(&mut app);
    app
}

/// The three test options, all distinct.
fn options() -> Vec<DropdownOption<TestChoice>> {
    vec![
        DropdownOption::new(TestChoice::new("alpha"), "Alpha"),
        DropdownOption::new(TestChoice::new("bravo"), "Bravo"),
        DropdownOption::new(TestChoice::new("charlie"), "Charlie"),
    ]
}

/// Spawns a dropdown with the three options (initially showing the first) and settles the
/// layout, returning the control entity.
fn spawn_test_dropdown(app: &mut App) -> Entity {
    let control = {
        let mut commands = app.world_mut().commands();
        spawn_dropdown(&mut commands, options(), 0, COLORS, ())
    };
    for _ in 0..3 {
        app.update();
    }
    control
}

/// Presses an entity (sets `Interaction::Pressed`) and runs ONLY the `Update` schedule so the
/// matching `Changed<Interaction>==Pressed` driver fires on the press edge.
///
/// It deliberately runs `Update` directly rather than `app.update()`: under the
/// `DefaultPlugins` headless harness the primary window is absent, so `bevy_ui`'s built-in
/// `ui_focus_system` (in `PreUpdate`) `set_if_neq`s every node's `Interaction` back to `None`
/// (no cursor over any node) — a full `app.update()` would clobber the manually-set `Pressed`
/// BEFORE the dropdown driver reads it. Running only `Update` (where every dropdown driver
/// lives) reads the edge we set; [`settle`] then runs full updates for layout.
fn press(app: &mut App, entity: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(entity) {
        *interaction = Interaction::Pressed;
    }
    app.world_mut().run_schedule(Update);
}

/// Runs a few full `app.update()`s so freshly-spawned UI nodes get a computed layout. (These
/// re-clobber transient `Interaction`s to `None`, which is fine once a press has been handled.)
fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

/// The single open `DropdownPopup` entity, if any.
fn popup_of(app: &mut App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<DropdownPopup>>();
    q.iter(app.world()).next()
}

/// The single open `DropdownBackdrop` entity, if any.
fn backdrop_of(app: &mut App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<DropdownBackdrop>>();
    q.iter(app.world()).next()
}

/// The option-row entities of the open popup, in child order.
fn option_rows(app: &mut App) -> Vec<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<DropdownItem<TestChoice>>>();
    let mut rows: Vec<Entity> = q.iter(app.world()).collect();
    rows.sort_by_key(|&e| {
        app.world()
            .get::<DropdownItem<TestChoice>>(e)
            .map_or(usize::MAX, |item| *item.index())
    });
    rows
}

/// The shown label text of the closed control.
fn shown_label(app: &App, control: Entity) -> Option<String> {
    let children = app.world().get::<Children>(control)?;
    for &child in children {
        if app.world().get::<DropdownLabel>(child).is_some() {
            return app.world().get::<Text>(child).map(|t| t.0.clone());
        }
    }
    None
}

/// Drains the `DropdownSelectionChanged<TestChoice>` messages.
fn selection_messages(app: &mut App) -> Vec<DropdownSelectionChanged<TestChoice>> {
    let mut state: bevy::ecs::system::SystemState<
        MessageReader<DropdownSelectionChanged<TestChoice>>,
    > = bevy::ecs::system::SystemState::new(app.world_mut());
    let Ok(mut reader) = state.get_mut(app.world_mut()) else {
        return Vec::new();
    };
    reader.read().cloned().collect()
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
    if let Some(mut keys) = app.world_mut().get_resource_mut::<ButtonInput<KeyCode>>() {
        keys.press(key);
    }
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

/// The number of laid-out option rows (carrying the non-generic [`DropdownItemMarker`]) in the
/// world — zero when no list is open, one per option while open.
fn option_marker_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<DropdownItemMarker>>();
    q.iter(app.world()).count()
}

/// The [`BackgroundColor`] of `entity`, if any.
fn background_of(app: &App, entity: Entity) -> Option<Color> {
    app.world().get::<BackgroundColor>(entity).map(|bg| bg.0)
}

/// The [`ComputedStackIndex`] of `entity` (its back-to-front paint order — higher = nearer the
/// front), if laid out. In Bevy 0.19 the paint order lives on the `ComputedStackIndex` component,
/// not on `ComputedNode`.
fn stack_index_of(app: &App, entity: Entity) -> Option<u32> {
    app.world().get::<ComputedStackIndex>(entity).map(|i| i.0)
}

/// GTW-499 C2: while the list is OPEN, the HIGHLIGHTED option row (the currently-selected /
/// hovered / focused row) is present, laid out with a non-zero size, and carries the styled
/// `option_highlight_bg` — NOT the resting `option_bg`. And it sits ON TOP (a higher
/// `ComputedNode.stack_index` than the popup container — occlusion-aware, bevy-traps #8).
///
/// The dropdown opens having focused option 0 AND option 0 is the selected index, so row 0 is the
/// highlighted row; the OTHER rows must stay on the resting `option_bg`. Pin-discriminating: if the
/// highlight painter were removed (or the shared `theme_interaction` were left to clobber the
/// option rows), row 0 would NOT carry `option_highlight_bg` (it would show `option_bg` or the
/// global theme fill), so the highlight assert fails; if no highlight color were applied, the
/// "distinct from `option_bg`" assert fails.
#[test]
fn open_dropdown_highlights_the_selected_option_row() {
    let mut app = harness();
    let control = spawn_test_dropdown(&mut app);

    press(&mut app, control);
    // Settle so the freshly-spawned rows lay out AND the highlight painter runs over them.
    settle(&mut app);

    let rows = option_rows(&mut app);
    assert_eq!(rows.len(), 3, "must have opened a 3-row list");
    let highlighted = rows[0]; // option 0 is both focused-on-open and the selected index

    // (a) The highlight row exists and (b) is laid out with a non-zero size.
    let size = app
        .world()
        .get::<ComputedNode>(highlighted)
        .map(ComputedNode::size);
    assert!(
        size.is_some_and(|s| s.x > 0.0 && s.y > 0.0),
        "the highlighted option row must be laid out with a non-zero ComputedNode size (got \
         {size:?})",
    );

    // (c) It carries the STYLED highlight fill — `option_highlight_bg`, NOT `option_bg`.
    assert_eq!(
        background_of(&app, highlighted),
        Some(COLORS.option_highlight_bg),
        "the highlighted option row must carry the dropdown's option_highlight_bg (the styled \
         highlight), NOT the resting option_bg or a default fill (GTW-499 C2)",
    );
    assert_ne!(
        COLORS.option_highlight_bg, COLORS.option_bg,
        "precondition: the highlight color must be DISTINCT from the resting option color",
    );

    // The non-highlighted rows keep the resting `option_bg`.
    for &row in &rows[1..] {
        assert_eq!(
            background_of(&app, row),
            Some(COLORS.option_bg),
            "a non-highlighted option row must keep the resting option_bg (no stray highlight)",
        );
    }

    // Occlusion-aware: the highlighted row paints ON TOP of (or with) the popup container — it is a
    // front node, not painted under (bevy-traps #8). The popup carries the above-panels z that
    // propagates to its rows, so the row's stack index is >= the popup's.
    let popup = popup_of(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let row_z = stack_index_of(&app, highlighted);
    let popup_z = stack_index_of(&app, popup);
    assert!(
        row_z.is_some() && popup_z.is_some() && row_z >= popup_z,
        "the highlighted option row must be a FRONT node (stack_index {row_z:?} >= popup \
         {popup_z:?}), so its highlight is not occluded (bevy-traps #8)",
    );
}

/// GTW-499 C2 (the stray-bar regression): in the CLOSED / non-open state there is NO option-row
/// highlight node at all — no popup, no rows. So a highlight can never leak into the collapsed
/// control's row.
///
/// Pin-discriminating: the old layout's stray highlight bar lived in the collapsed row; this
/// asserts that with the list closed there are ZERO `DropdownItemMarker` rows (the highlight is
/// owned by the option rows, which only exist while the list is open).
#[test]
fn closed_dropdown_shows_no_option_highlight() {
    let mut app = harness();
    let control = spawn_test_dropdown(&mut app);

    // Precondition: closed — no option rows exist.
    assert_eq!(
        option_marker_count(&mut app),
        0,
        "a CLOSED dropdown must have NO option rows (so no stray highlight bar — GTW-499 C2)",
    );

    // Open then close (re-press the control) and confirm the rows — and any highlight — are gone.
    press(&mut app, control);
    settle(&mut app);
    assert_eq!(
        option_marker_count(&mut app),
        3,
        "opening shows the option rows",
    );

    press(&mut app, control);
    settle(&mut app);
    assert_eq!(
        option_marker_count(&mut app),
        0,
        "re-pressing the control closes the list, despawning EVERY option row — no highlight node \
         survives into the collapsed state (GTW-499 C2)",
    );
}
