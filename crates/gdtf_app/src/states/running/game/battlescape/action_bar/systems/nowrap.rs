//! [`nowrap_control_labels`] — keep the relocated control SEGMENTS' labels on ONE line AND at a
//! SMALL font so they fit their narrow segments WITHOUT clipping (GTW-298 / GTW-277).
//!
//! GTW-277 migrated the firemode + stance controls to `gdtf_ui`
//! [`SegmentedControl`](gdtf_ui::SegmentedControl)s. Their segments still live in narrow cells:
//! the firemode control is in a SHORT (bottom 1/4-height) cell whose per-segment width is a small
//! share of the panel (the contract's "width varies by count"). At that width a long mode label
//! (e.g. `full-auto`) is WIDER than its segment at the widget's segment font, and `bevy_ui`'s
//! default text layout SOFT-WRAPS it — which overflows the short cell and reads as the firemode
//! segments OVERLAPPING the row above (contract item 8: no control overlaps another). The
//! full-auto mode's DISPLAYED label is shortened to "auto" (see `mode_label`); this system flips
//! every relocated-control segment label to [`LineBreak::NoWrap`] (so a too-wide caption stays on
//! ONE line) and sets its [`TextFont::font_size`](bevy::text::TextFont) to the GTW-298
//! comfortably-legible [`CONTROL_LABEL_PT`] (14 pt) so the captions fit WITHOUT clipping.
//!
//! `gdtf_ui::spawn_segmented_control` spawns each segment label as a child [`Text`] with the
//! default wrap, at the widget's own segment font size. This system, per relocated control
//! segment, flips its label to [`LineBreak::NoWrap`] and shrinks its
//! [`TextFont::font_size`](bevy::text::TextFont) to [`CONTROL_LABEL_PT`], AND tightens the
//! [`ModePanelRoot`] panel's L/R [`padding`](bevy::ui::Node) to [`MODE_PANEL_PAD_VW`] (so the
//! narrow firemode row spends the cell width on captions, not the theme's wide panel inset). It
//! does NOT touch the shared `gdtf_ui` widget code nor the theme: it post-processes only the
//! relocated firemode / stance segment markers. It runs `.after(UiSystems::ApplyTheme)` and writes
//! only on a real change, so a repaint that re-applies the widget font / theme padding is
//! re-corrected the next frame. NOTE the Aim control is now a knob-only `Switch` (no text label),
//! so it is no longer a target of this label-sizing pass.

use bevy::{
    prelude::*,
    text::{FontSize, LineBreak, TextFont},
    ui::{Node, Val},
};

use crate::states::running::game::battlescape::action_bar::components::{
    ModeBurstButton, ModeFullButton, ModePanelRoot, ModeSingleButton, StanceKneelingButton,
    StanceProneButton, StanceStandingButton,
};

/// The font size (pt) the relocated control-toggle labels are shrunk to so the firemode
/// captions ("single" / "burst" / "full-auto") fit their narrow toggles without clipping.
///
/// A named newtype over the size rather than a bare `f32` (no-bare-types rule): it is a
/// presentation font size, the `ModeGapVw` plumbing-newtype precedent. Smaller than the theme's
/// 18 pt button font; applied per-label here so the theme (and every other button) is untouched.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ControlLabelPt(f32);

/// The relocated control-toggle label size: 14 pt — comfortably legible (matching the weapon
/// cluster's `WEAPON_LABEL_PT`), and small enough that every firemode caption (`single` /
/// `burst` / the renamed `auto`), stance caption (`Stand` / `Kneel` / `Prone`), and the `Aim`
/// caption fits its toggle without clipping under [`LineBreak::NoWrap`] + the panel's
/// `overflow: Hidden` clip. Raised from 9 pt, which the screenshot review (2026-06-18) found
/// far too small to read.
const CONTROL_LABEL_PT: ControlLabelPt = ControlLabelPt(14.0);

/// The tightened LEFT/RIGHT padding of a relocated control panel, as a fraction of the viewport
/// WIDTH ([`Val::Vw`](bevy::ui::Val::Vw)), replacing the theme's panel margin so the narrow
/// firemode row spends its width on the captions.
///
/// A named newtype over the padding rather than a bare `f32` (no-bare-types rule): layout
/// spacing, the `ModeGapVw` precedent. Horizontal padding, so the unit is `Vw`
/// (`ui-responsive-not-px`). Applied per-panel here so the theme is untouched.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ControlPadVw(f32);

/// The Firemode panel's tightened LEFT/RIGHT padding: 0.15625 vw (2 px at the 1280-wide reference
/// window) — the theme paints the [`ModePanelRoot`] (a `Themed(Panel)`) with its panel margin,
/// which steals the segment row's width and clips the widest caption (`full-auto`). Tightening the
/// panel's own L/R inset to a hairline gives the segment row back the cell width so every caption
/// fits. Applied per-panel here so the theme is untouched.
const MODE_PANEL_PAD_VW: ControlPadVw = ControlPadVw(0.15625);

/// Query FILTER selecting the relocated control SEGMENTS whose label should not wrap — the
/// firemode + stance segments (GTW-277 / GTW-298). An `Or` over their per-segment markers so one
/// query reaches every relocated control segment. (The Aim control is a knob-only `Switch` with no
/// label, so it is not included.)
///
/// Aliased so the [`nowrap_control_labels`] segment query type stays legible (clippy
/// `type_complexity`).
type RelocatedControlSegment = (
    Or<(
        With<ModeSingleButton>,
        With<ModeBurstButton>,
        With<ModeFullButton>,
        With<StanceStandingButton>,
        With<StanceKneelingButton>,
        With<StanceProneButton>,
    )>,
    // Disjointness proof for the `mode_panels` `&mut Node` query: the Firemode PANEL ROOT
    // (`ModePanelRoot`) carries none of the segment markers, so excluding it here proves the
    // segment-children read and the `mode_panels` `&mut Node` write never alias.
    Without<ModePanelRoot>,
);

/// Keeps the relocated firemode / stance segment LABELS on one line ([`LineBreak::NoWrap`]) AND at
/// the small [`CONTROL_LABEL_PT`] (14 pt) font, AND tightens the Firemode panel's L/R padding to
/// [`MODE_PANEL_PAD_VW`], so a too-wide caption fits its narrow segment without clipping (GTW-298
/// item 8 / GTW-277).
///
/// For each segment carrying a relocated-control marker ([`RelocatedControlSegment`]) it walks the
/// segment's [`Children`] and, for any child that is a [`Text`] label, sets that label's
/// [`TextLayout`] linebreak to [`LineBreak::NoWrap`] AND its
/// [`TextFont::font_size`](bevy::text::TextFont) to [`CONTROL_LABEL_PT`] — each write only when it
/// differs (change-detection hygiene). `gdtf_ui`'s `repaint_segments` re-derives only the label
/// WEIGHT + COLOR on an active change, NOT its `font_size`, so this 14 pt size survives the
/// segment-repaint. The segments are spawned once and never churned, so this settles after spawn
/// and is a no-op thereafter. Runs under the action-bar plugin's live-battle gate,
/// `.after(UiSystems::ApplyTheme)`.
///
/// It ALSO tightens the Firemode panel root's ([`ModePanelRoot`]) own L/R padding to
/// [`MODE_PANEL_PAD_VW`]: the theme paints that panel with its panel margin, which steals the
/// segment row's width and clips the widest caption — so the panel inset is tightened to a hairline
/// to give the segment row back the cell width.
///
/// Param-only (`bevy-traps.md` #7): a read-only `Query<&Children, RelocatedControlSegment>` over
/// the segment ROOTS, a `&mut Node` write query over the [`ModePanelRoot`] (disjoint from the
/// segment query — the panel root carries none of the segment markers), and a
/// `Query<(&mut TextLayout, &mut TextFont), With<Text>>` over the label CHILDREN — no [`Commands`],
/// no `&mut World`. The segment-children read / panel-root `Node` write / label `Text` write are
/// disjoint (the label is a child entity), so they never conflict.
pub(in crate::states::running::game::battlescape) fn nowrap_control_labels(
    segments: Query<&Children, RelocatedControlSegment>,
    mut mode_panels: Query<&mut Node, With<ModePanelRoot>>,
    mut labels: Query<(&mut TextLayout, &mut TextFont), With<Text>>,
) {
    // Tighten the Firemode panel's own L/R padding so its segment row uses the full cell width
    // (the theme re-applies its panel margin every repaint; we override only L/R here).
    for mut node in &mut mode_panels {
        if node.padding.left != Val::Vw(*MODE_PANEL_PAD_VW) {
            node.padding.left = Val::Vw(*MODE_PANEL_PAD_VW);
        }
        if node.padding.right != Val::Vw(*MODE_PANEL_PAD_VW) {
            node.padding.right = Val::Vw(*MODE_PANEL_PAD_VW);
        }
    }
    for children in &segments {
        for &child in children {
            let Ok((mut layout, mut font)) = labels.get_mut(child) else {
                continue;
            };
            if layout.linebreak != LineBreak::NoWrap {
                layout.linebreak = LineBreak::NoWrap;
            }
            if font.font_size != FontSize::Px(*CONTROL_LABEL_PT) {
                font.font_size = FontSize::Px(*CONTROL_LABEL_PT);
            }
        }
    }
}
