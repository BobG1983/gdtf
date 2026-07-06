//! GTW-428 layout fit: the expanded panel holds all sixteen lines above the old clip.

use bevy::{
    app::App,
    ecs::{component::Component, entity::Entity, hierarchy::Children},
    ui::{Interaction, Node, Val},
};
use gdtf_app::test_support::{AttributeField, DerivedStatText, ExpandPip, MemberStatPanel};
use gdtf_ui::AccordionAnim;

use super::harness::*;

/// Whether the panel's `Node.height` is [`Val::Auto`] — the CONTENT-FIT rest-open height the
/// shared `drive_accordions` switches a settled [`AccordionContentFit`] section to, so the
/// rest-open panel sizes to its exact content (the GTW-428 round-2 layout fix).
fn panel_height_is_auto(app: &App, panel: Entity) -> bool {
    matches!(
        app.world().get::<Node>(panel).map(|node| node.height),
        Some(Val::Auto)
    )
}

/// All descendant entities of `root` (depth-first, `root` excluded) — the panel's full subtree, so
/// the layout-guard can confirm every stat line is REACHABLE under the panel content (C3).
fn descendants(app: &App, root: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(children) = app.world().get::<Children>(entity) {
            for &child in children {
                out.push(child);
                stack.push(child);
            }
        }
    }
    out
}

/// The line-marker entities of kind `M` (an [`AttributeField`] / [`DerivedStatText`] marker) that
/// are descendants of `root` — i.e. laid out UNDER the panel content, reachable (C3).
fn marked_descendants<M: Component>(app: &App, root: Entity) -> Vec<Entity> {
    descendants(app, root)
        .into_iter()
        .filter(|&entity| app.world().get::<M>(entity).is_some())
        .collect()
}

/// GTW-428 layout-guard (the live-feature defect fix): after the pip opens the row-0 stat panel and
/// the accordion lerp SETTLES, ALL SIXTEEN stat lines (eight `AttributeField` + eight
/// `DerivedStatText`) are laid out and reachable under the panel content, AND the rest-open panel
/// sizes its height to FIT its content rather than clipping to a fixed `Vh` ceiling (the old shared
/// 18vh default — and the round-1 fixed 38vh — both clipped the taller column's bottom lines).
///
/// Concretely it asserts three things the pre-fix code fails:
///
/// 1. The settled panel's height is [`Val::Auto`] — the CONTENT-FIT rest height the generalized
///    accordion switches an [`AccordionContentFit`] section to once it settles fully open, so the
///    panel sizes to its exact content regardless of font metrics. The pre-fix code (and the
///    round-1 fixed-`Vh` attempt) settled to a `Val::Vh` ceiling and fails this `is Auto` assert.
/// 2. The panel lays its lines in EXACTLY TWO columns, each a direct child holding eight of the
///    sixteen lines (the compact 2-column layout). Pre-fix the sixteen lines hang in a single
///    column and the two-equal-columns assert fails.
/// 3. All eight `AttributeField` AND all eight `DerivedStatText` markers are reachable descendants
///    of the panel content (none orphaned by the restructure).
///
/// Pin-discriminating: reverting to a fixed-`Vh` settled height (dropping `AccordionContentFit`)
/// fails assert 1; reverting the 2-column split (back to one column of sixteen) fails assert 2;
/// dropping any line fails assert 3. The existing GTW-428 tests
/// (`pip_press_drives_the_accordion_target`, `expanded_panel_has_eight_clamped_attribute_fields`,
/// `editing_attribute_recomputes_derived_to_pipeline_output`) and the GTW-416 accordion tests stay
/// green — they assert behavior this fix preserves.
#[test]
fn expanded_panel_fits_all_sixteen_lines_above_the_old_clip() {
    let mut app = editor_app();
    press_add_member(&mut app);

    let pip = control_for_row::<ExpandPip>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);
    let panel = control_for_row::<MemberStatPanel>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    // Open the panel (the headless idiom — set Pressed then update), then let the lerp settle.
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(pip) {
        *interaction = Interaction::Pressed;
    }
    app.update();
    // Settle the lerp: the harness steps `Time` a fixed delta per update, so a generous run reaches
    // the per-instance expanded target and snaps to it.
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        app.world().get::<AccordionAnim>(panel),
        Some(&AccordionAnim::Expanded),
        "the panel must settle fully open (Expanded) before the layout is asserted",
    );

    // Assert 1 — the settled panel switched to a CONTENT-FIT (`Val::Auto`) height, so it sizes to
    // its exact content rather than clipping to a fixed `Vh` ceiling (the old 18vh default, and the
    // round-1 fixed 38vh, both clipped the taller column's bottom lines at the live window size).
    assert!(
        panel_height_is_auto(&app, panel),
        "the settled open panel must adopt a content-fit Val::Auto height (the AccordionContentFit \
         rest behavior) so all sixteen lines fit regardless of font metrics — got height {:?}",
        app.world().get::<Node>(panel).map(|node| node.height),
    );

    // Assert 2 — the panel holds EXACTLY two columns, each a direct child.
    let columns: Vec<Entity> = app
        .world()
        .get::<Children>(panel)
        .map(|children| children.iter().copied().collect())
        .unwrap_or_default();
    assert_eq!(
        columns.len(),
        2,
        "the panel must lay its sixteen lines in EXACTLY two columns (the compact 2-column layout); \
         got {} direct children",
        columns.len(),
    );
    // Each column carries eight of the sixteen lines (8 attributes in one, 8 derived in the other).
    let mut per_column_line_counts: Vec<usize> = columns
        .iter()
        .map(|&column| {
            let attrs = marked_descendants::<AttributeField>(&app, column).len();
            let derived = marked_descendants::<DerivedStatText>(&app, column).len();
            attrs + derived
        })
        .collect();
    per_column_line_counts.sort_unstable();
    assert_eq!(
        per_column_line_counts,
        vec![8, 8],
        "each of the two columns must hold eight of the sixteen stat lines (got {per_column_line_counts:?})",
    );

    // Assert 3 — all sixteen line markers are reachable descendants of the panel content.
    let attribute_lines = marked_descendants::<AttributeField>(&app, panel);
    let derived_lines = marked_descendants::<DerivedStatText>(&app, panel);
    assert_eq!(
        attribute_lines.len(),
        8,
        "all eight editable AttributeField lines must be laid out under the open panel (C3); got {}",
        attribute_lines.len(),
    );
    assert_eq!(
        derived_lines.len(),
        8,
        "all eight readonly DerivedStatText lines must be laid out under the open panel (C3); got {}",
        derived_lines.len(),
    );
}
