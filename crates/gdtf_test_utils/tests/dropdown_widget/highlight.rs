//! Selected-row highlight: open shows it, closed shows none.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, ComputedNode, ComputedStackIndex},
};
use gdtf_ui::DropdownItemMarker;

use super::harness::*;

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
