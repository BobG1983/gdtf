//! The TU / HP bar furniture: the `cur/max` numeric label and the label-over-bar column
//! wrapper (the GTW-310 contrast/overflow rework). Split out of the monolithic
//! `build.rs` (GTW-583); the builder rationale lives on the parent `build` module.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Node, Val},
};
use gdtf_ui::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::block::ROW_GAP_VH;

/// The font size of the `cur/max` numeric label sitting ABOVE its TU / HP bar, in points.
///
/// Matches the body-text size so the number reads cleanly on the DARK panel background
/// (the mockup pairing — `cur/max` shares the line/size of the caption it sits with), a
/// legible size, not a tiny overlay. `font_size` in points is the ONE permitted
/// bare-`f32` exception to `ui-responsive-not-px` (a typographic size, not a layout
/// dimension); a `const` so the label size lives in one place (GTW-310 rework).
const BAR_LABEL_FONT_PT: f32 = 12.0;

/// Spawns the `cur/max` numeric [`Text`](bevy::prelude::Text) for a TU / HP bar, carrying
/// its `marker`, and returns its [`Entity`].
///
/// Unlike the original GTW-310 overlay, the label is now a NORMAL-FLOW line that the
/// [`spawn_bar_group`] wrapper stacks ABOVE its bar, so the number sits on the DARK panel
/// background (not on the bright bar fill) where the [theme text color](GdtfTheme) reads
/// with contrast — the same light color the name / faction / stance lines use, which read
/// cleanly on the dark panel — and where it has room rather than overflowing the thin
/// track (GTW-310 rework). It is a [`Themed(ThemeRole::Text)`](Themed) line so
/// `apply_theme` restyles it on a theme change, at the legible [`BAR_LABEL_FONT_PT`]; the
/// text is right-justified so the number aligns to the bar's right edge (the mockup
/// pairing). Spawned empty; the per-update
/// [`update_stat_block`](super::super::update_stat_block) writes `"cur/max"` in place
/// ([[ui-mutate-not-respawn]]).
pub(super) fn spawn_bar_label(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
) -> Entity {
    // GTW-322 — `Themed` + `Text::new` + `UiTextColor` ride the `bsn!` macro inline;
    // `TextFont` (not `Unpin`) rides the `template(|_| ..)` closure; the runtime-valued
    // `TextLayout` (right-justify) and full-width `Node` are composed with `template_value`;
    // the generic `marker` is `.insert`ed after the scene is queued.
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(BAR_LABEL_FONT_PT),
        ..default()
    };
    // A full-width line so the right-justified number anchors to the bar's right edge
    // above it; the column wrapper gives it a row of its own (no overflow).
    let node = Node {
        width: Val::Percent(100.0),
        ..default()
    };
    commands
        .spawn_scene((
            bsn! {
                Themed::new(ThemeRole::Text)
                Text::new("")
                UiTextColor(text_color)
                template(move |_| Ok(text_font.clone()))
            },
            template_value(TextLayout::justify(Justify::Right)),
            template_value(node),
        ))
        .insert(marker)
        .id()
}

/// Wraps a TU / HP `cur/max` `label` and its `bar` in a tight vertical column (label ON
/// TOP, bar below) and returns the wrapper [`Entity`].
///
/// This is the GTW-310 rework's contrast/overflow fix: the number rides on the DARK panel
/// background ABOVE the bar — off the bright bar fill — so it reads cleanly and has room.
/// The wrapper is a plain layout [`Node`] (no [`Themed`] marker, so `apply_theme` leaves
/// it alone) that consumes the bar's full width; the bar keeps its own
/// [`StatTuBar`](super::super::components::StatTuBar) /
/// [`StatHpBar`](super::super::components::StatHpBar) marker + its
/// [`ProgressBarFill`](gdtf_ui::ProgressBarFill) child untouched, so the per-update fill
/// query still finds it by stored id. A small `row_gap` keeps the number off the bar's top
/// edge.
pub(super) fn spawn_bar_group(commands: &mut Commands, label: Entity, bar: Entity) -> Entity {
    // GTW-322 — a plain layout `Node` (no markers); composed via `template_value`.
    let node = Node {
        flex_direction: FlexDirection::Column,
        width: Val::Percent(100.0),
        row_gap: Val::Vh(ROW_GAP_VH),
        ..default()
    };
    let group = commands.spawn_scene(template_value(node)).id();
    commands.entity(group).add_children(&[label, bar]);
    group
}
