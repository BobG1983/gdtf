//! [`nowrap_control_labels`] — keep the relocated control toggles' labels on ONE line AND at a
//! SMALL font so they fit their narrow toggles WITHOUT clipping (GTW-298).
//!
//! After GTW-298 relocated the firemode / aim / stance toggles into the weapon cluster, the
//! firemode toggles live in a SHORT (bottom 1/4-height) cell whose per-toggle width is a small
//! share of the panel (the contract's "width varies by count"). At that width a long mode label
//! (e.g. `full-auto`) is WIDER than its toggle at the theme's 18 pt button font, and `bevy_ui`'s
//! default text layout SOFT-WRAPS it onto a second line — which then overflows the short cell and
//! reads as the firemode buttons OVERLAPPING the row above (contract item 8: no control overlaps
//! another). The first GTW-298 pass set [`LineBreak::NoWrap`] so the caption stayed on one line,
//! but at 18 pt it still CLIPPED ("single" → "ingl", "full-auto" → "ll-au"). An interim pass
//! dropped the label font to 9 pt, which the next screenshot review (2026-06-18) found far too
//! small to read; this pass raises it to a comfortably-legible [`CONTROL_LABEL_PT`] and the
//! full-auto mode's DISPLAYED label is shortened to "auto" (see `spawn_hidden_toggle`) so every
//! caption fits at the larger size WITHOUT clipping.
//!
//! `gdtf_ui::spawn_button` spawns each label as a child [`Text`] with the default
//! ([`LineBreak::WordBoundary`](bevy::text::LineBreak::WordBoundary)) wrap, at the theme's button
//! font size, and the button root carries the theme's 12 px L/R button padding. This system, per
//! relocated control toggle, flips its label to [`LineBreak::NoWrap`] (so a too-wide caption stays
//! on ONE line), shrinks its [`TextFont::font_size`](bevy::text::TextFont) to
//! [`CONTROL_LABEL_PT`], AND tightens the toggle root's L/R [`padding`](bevy::ui::Node) to
//! [`CONTROL_PAD_VW`] (so the narrow firemode toggles spend their width on the caption, not the
//! theme's wide inset). It does NOT touch the shared `gdtf_ui` widget (the migration to generic
//! `Switch` / `SegmentedControl` widgets is still GTW-277) nor the theme (which would resize /
//! re-pad EVERY button app-wide): it post-processes only the firemode / aim / stance markers. It
//! runs `.after(UiSystems::ApplyTheme)` and writes only on a real change, so a theme repaint that
//! re-applies the 18 pt font / 12 px padding is re-corrected the next frame.

use bevy::{
    prelude::*,
    text::{LineBreak, TextFont},
    ui::{Node, Val},
};

use crate::scenes::running::game::battlescape::action_bar::components::{
    AimToggleButton, ModeBurstButton, ModeFullButton, ModePanelRoot, ModeSingleButton,
    StanceKneelingButton, StanceProneButton, StanceStandingButton,
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

/// The tightened LEFT/RIGHT padding of a relocated control toggle, as a fraction of the viewport
/// WIDTH ([`Val::Vw`](bevy::ui::Val::Vw)), replacing the theme's button margin so the narrow
/// firemode toggles spend their width on the caption.
///
/// A named newtype over the padding rather than a bare `f32` (no-bare-types rule): layout
/// spacing, the `ModeGapVw` precedent. Horizontal padding, so the unit is `Vw`
/// (`ui-responsive-not-px`). Applied per-toggle here so the theme is untouched.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ControlPadVw(f32);

/// The relocated control-toggle's tightened horizontal padding: 0.234375 vw (3 px at the
/// 1280-wide reference window — a tight inset so even the widest firemode caption fits the narrow
/// toggle).
const CONTROL_PAD_VW: ControlPadVw = ControlPadVw(0.234_375);

/// The Firemode panel's tightened LEFT/RIGHT padding: 0.15625 vw (2 px at the 1280-wide reference
/// window) — the theme paints the [`ModePanelRoot`] (a `Themed(Panel)`) with its panel margin,
/// which stole the toggle row's width and forced the widest caption (`full-auto`) to clip
/// ("full-autc") even at the small [`CONTROL_LABEL_PT`] font (R2 / NR6). Tightening the panel's
/// own L/R inset to a hairline gives the toggle row back the cell width so every caption fits. A
/// `ControlPadVw` reuse (the same plumbing-newtype), applied per-panel here so the theme is
/// untouched.
const MODE_PANEL_PAD_VW: ControlPadVw = ControlPadVw(0.15625);

/// Query FILTER selecting the relocated control toggles whose label should not wrap — the
/// firemode, aim, and stance toggles (GTW-298). An `Or` over their markers so one query reaches
/// every relocated control button.
///
/// Aliased so the [`nowrap_control_labels`] toggle query type stays legible (clippy
/// `type_complexity`).
type RelocatedControlToggle = (
    Or<(
        With<ModeSingleButton>,
        With<ModeBurstButton>,
        With<ModeFullButton>,
        With<AimToggleButton>,
        With<StanceStandingButton>,
        With<StanceKneelingButton>,
        With<StanceProneButton>,
    )>,
    // Disjointness proof for the second `&mut Node` query below: the Firemode PANEL ROOT
    // (`ModePanelRoot`) carries none of the toggle markers, so excluding it here proves the
    // toggle-root `&mut Node` query and the `mode_panels` `&mut Node` query never alias.
    Without<ModePanelRoot>,
);

/// Keeps the relocated firemode / aim / stance toggle LABELS on one line
/// ([`LineBreak::NoWrap`]) AND at the small [`CONTROL_LABEL_PT`] font, AND tightens each toggle's
/// L/R padding to [`CONTROL_PAD_VW`], so a too-wide caption fits its narrow toggle without
/// clipping (GTW-298, contract item 8 + screenshot review 2026-06-18).
///
/// For each toggle carrying a relocated-control marker ([`RelocatedControlToggle`]) it tightens the
/// toggle root's L/R [`padding`](bevy::ui::Node) to [`CONTROL_PAD_VW`], then walks the toggle's
/// [`Children`] and, for any child that is a [`Text`] label, sets that label's [`TextLayout`]
/// linebreak to [`LineBreak::NoWrap`] AND its [`TextFont::font_size`](bevy::text::TextFont) to
/// [`CONTROL_LABEL_PT`] — each write only when it differs (change-detection hygiene). The toggles
/// are spawned once and never churned, so this settles in the first frame after the panel spawns
/// and is a no-op thereafter; a theme repaint that re-applies the theme's 18 pt font / 12 px
/// padding is re-corrected the next frame (the system runs every frame under the live-battle gate).
/// Runs under the action-bar plugin's live-battle gate, `.after(UiSystems::ApplyTheme)`.
///
/// It ALSO tightens the Firemode panel root's ([`ModePanelRoot`]) own L/R padding to
/// [`MODE_PANEL_PAD_VW`] (GTW-298 R2 / NR6): the theme paints that panel with its 12 px panel
/// margin, which stole the toggle row's width and clipped the widest caption (`full-auto` →
/// `full-autc`) even at the small label font — so the panel inset is tightened to a hairline to
/// give the toggle row back the cell width.
///
/// Param-only (`bevy-traps.md` #7): a read-only `Query<&Children, RelocatedControlToggle>`, a
/// `&mut Node` write query over the toggle ROOTS, a `&mut Node` write query over the
/// [`ModePanelRoot`] (disjoint from the toggle query — the panel root carries none of the toggle
/// markers), and a `Query<(&mut TextLayout, &mut TextFont), With<Text>>` over the label CHILDREN —
/// no [`Commands`], no `&mut World`. The toggle-root / panel-root `Node` queries and the label
/// `Text` query are disjoint (the label is a child entity), so they never conflict.
pub(in crate::scenes::running::game::battlescape) fn nowrap_control_labels(
    mut toggles: Query<(&Children, &mut Node), RelocatedControlToggle>,
    mut mode_panels: Query<&mut Node, With<ModePanelRoot>>,
    mut labels: Query<(&mut TextLayout, &mut TextFont), With<Text>>,
) {
    // Tighten the Firemode panel's own L/R padding so its toggle row uses the full cell width
    // (the theme re-applies its 12 px panel margin every repaint; we override only L/R here).
    for mut node in &mut mode_panels {
        if node.padding.left != Val::Vw(*MODE_PANEL_PAD_VW) {
            node.padding.left = Val::Vw(*MODE_PANEL_PAD_VW);
        }
        if node.padding.right != Val::Vw(*MODE_PANEL_PAD_VW) {
            node.padding.right = Val::Vw(*MODE_PANEL_PAD_VW);
        }
    }
    for (children, mut node) in &mut toggles {
        // Tighten the toggle root's L/R padding (the theme re-applies its 12 px button margin
        // every repaint; we override only L/R so the narrow firemode toggle spends its width on
        // the caption).
        if node.padding.left != Val::Vw(*CONTROL_PAD_VW) {
            node.padding.left = Val::Vw(*CONTROL_PAD_VW);
        }
        if node.padding.right != Val::Vw(*CONTROL_PAD_VW) {
            node.padding.right = Val::Vw(*CONTROL_PAD_VW);
        }
        for &child in children {
            let Ok((mut layout, mut font)) = labels.get_mut(child) else {
                continue;
            };
            if layout.linebreak != LineBreak::NoWrap {
                layout.linebreak = LineBreak::NoWrap;
            }
            if font.font_size != *CONTROL_LABEL_PT {
                font.font_size = *CONTROL_LABEL_PT;
            }
        }
    }
}
