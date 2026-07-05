//! Caller-driven `set_*` mutators: the GTW-284 per-segment visibility toggle and the
//! GTW-303 sub-line set / update / clear — all in-place mutation, never respawn.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{EntityCommandsSceneExt, bsn_list},
    text::TextColor as UiTextColor,
    ui::{Display, Node},
};

use super::{
    style::{sub_line_color, sub_line_font},
    types::{Segment, SegmentColors, SegmentIndex, SegmentSubLabel, SegmentSubText},
};

/// Shows or hides a single segment of a [`SegmentedControl`](super::SegmentedControl) BY INDEX,
/// MUTATING the
/// segment's [`Node::display`](bevy::ui::Node) in place — never despawning/respawning it
/// ([[ui-mutate-not-respawn]]).
///
/// A hidden segment is set to [`Display::None`], so the flex row/column COLLAPSES it: the
/// control visibly shows only the still-[`Display::Flex`] segments, with no gap left where
/// a hidden one was. This is the per-segment visibility a caller needs when a
/// [`SegmentedControl`](super::SegmentedControl)'s segments map to an OFFERED set that varies
/// (e.g. a weapon's
/// fire modes) yet the segment entity ids must stay STABLE — the GTW-284 mutate-not-churn
/// invariant: the control is spawned once with ALL segments, and the offered subset is
/// revealed by toggling per-segment visibility rather than rebuilding the control.
///
/// It walks the control's `children` for the [`Segment`] whose [`SegmentIndex`] equals
/// `index` and sets its [`Display`]; an out-of-range index is a no-op. The write is
/// guarded so it marks the [`Node`] changed only on a REAL change (change-detection
/// hygiene). Returns whether a matching segment was found.
///
/// Caller-driven (the `set_*` helper idiom — no per-frame system; the bar / pips
/// precedent): build a
/// `SystemState<(Query<&Children>, Query<(&SegmentIndex, &mut Node), With<Segment>>)>`,
/// call this, then `state.apply(world)`. Param-only — no `&mut World` (bevy-traps rule 7).
pub fn set_segment_visible(
    control: Entity,
    index: usize,
    visible: bool,
    children: &Query<&Children>,
    segments: &mut Query<(&SegmentIndex, &mut Node), With<Segment>>,
) -> bool {
    let Ok(kids) = children.get(control) else {
        return false;
    };
    let want = if visible {
        Display::Flex
    } else {
        Display::None
    };
    for &child in kids {
        let Ok((seg_index, mut node)) = segments.get_mut(child) else {
            continue;
        };
        if **seg_index == index {
            if node.display != want {
                node.display = want;
            }
            return true;
        }
    }
    false
}

/// Read-only [`Query`] data identifying one segment when setting its sub-line: its
/// [`SegmentIndex`] and its [`Children`] (to look for an existing [`SegmentSubText`] node).
///
/// Named to keep [`set_segment_sub_line`]'s signature legible (clippy `type_complexity`).
type SubLineSegment = (&'static SegmentIndex, &'static Children);

/// Sets, updates, or clears a single segment's OPTIONAL sub-line BY INDEX (GTW-303),
/// returning whether a matching segment was found.
///
/// The sub-line is a SECOND [`Text`](bevy::prelude::Text) node ([`SegmentSubText`]) STACKED
/// below the segment's primary [`SegmentText`](super::SegmentText) label, at a smaller font
/// ([`SEGMENT_SUB_FONT_PT`](super::style::SEGMENT_SUB_FONT_PT)) and a dimmer color
/// ([`sub_line_color`] of the segment's base
/// text) so it reads as a quiet secondary line. It is created / mutated / removed IN PLACE —
/// never by despawning/respawning the SEGMENT ([[ui-mutate-not-respawn]]):
///
/// - `Some(label)` and the segment has NO sub-line yet → SPAWNS the sub-line node as a child
///   of the segment (so it stacks below the label in the segment's centered column).
/// - `Some(label)` and the segment ALREADY has a sub-line → MUTATES that existing node's
///   [`Text`] in place to the new caption (the sub-line entity id stays STABLE), only when
///   the text actually differs (change-detection hygiene).
/// - `None` → DESPAWNS the segment's sub-line node if present (clears the sub-line), leaving
///   the segment rendering as a single centered label again.
///
/// A segment with no sub-line has NO [`SegmentSubText`] node, so it renders exactly as it did
/// before GTW-303 — the stance control and any other pre-GTW-303 caller are unaffected. The
/// sub-line's color is derived from the control root's [`SegmentColors::base_text`] (dimmed),
/// so it matches the segment palette without the caller re-passing colors.
///
/// Caller-driven (the `set_*` helper idiom — the bar / pips / [`set_segment_visible`]
/// precedent): build a
/// `SystemState<(Commands, Query<(&Children, &SegmentColors)>, Query<SubLineSegment, With<Segment>>, Query<&mut Text, With<SegmentSubText>>)>`,
/// call this, then `state.apply(world)`. Param-only — no `&mut World` (bevy-traps rule 7); the
/// spawn / despawn go through [`Commands`].
#[allow(
    clippy::type_complexity,
    reason = "param tuple aliased where possible; the spawn/mutate/clear paths fix the \
    children + colors + sub-text query shapes"
)]
pub fn set_segment_sub_line(
    commands: &mut Commands,
    control: Entity,
    index: usize,
    sub_line: Option<&SegmentSubLabel>,
    controls: &Query<(&Children, &SegmentColors)>,
    segments: &Query<SubLineSegment, With<Segment>>,
    sub_texts: &mut Query<&mut Text, With<SegmentSubText>>,
) -> bool {
    let Ok((kids, colors)) = controls.get(control) else {
        return false;
    };
    let dim = sub_line_color(colors.base_text);
    for &child in kids {
        let Ok((seg_index, seg_children)) = segments.get(child) else {
            continue;
        };
        if **seg_index != index {
            continue;
        }
        // The segment's existing sub-line node, if any (a SegmentSubText child).
        let existing = seg_children.iter().find(|&kid| sub_texts.get(kid).is_ok());
        match (sub_line, existing) {
            // Set / update: mutate an existing sub-line in place, or spawn a fresh one.
            (Some(label), Some(node)) => {
                if let Ok(mut text) = sub_texts.get_mut(node) {
                    let want = label.to_string();
                    if text.0 != want {
                        text.0 = want;
                    }
                }
            }
            (Some(label), None) => {
                let caption = label.to_string();
                let font = sub_line_font();
                // GTW-322 — author the sub-line text child as a `bsn!` scene parented
                // under the segment (mirrors the converted segment label + the
                // `SwitchKnob` child). `Text::new` + the unit `SegmentSubText` marker +
                // `UiTextColor(dim)` ride the inline macro; the `TextFont` is NOT `Unpin`
                // (its `FontFeatures` / `FontVariations` fields), so it takes the
                // `template(move |_| Ok(value.clone()))` closure escape hatch. It is
                // queued as a `Children` related-scene on the existing `child` segment, so
                // the lazy "spawn once if absent" semantics are preserved.
                commands
                    .entity(child)
                    .queue_spawn_related_scenes::<Children>(bsn_list! {
                        (
                            SegmentSubText
                            Text::new(caption)
                            UiTextColor(dim)
                            template(move |_| Ok(font.clone()))
                        )
                    });
            }
            // Clear: despawn the sub-line node if present; nothing to do otherwise.
            (None, Some(node)) => {
                commands.entity(node).despawn();
            }
            (None, None) => {}
        }
        return true;
    }
    false
}
