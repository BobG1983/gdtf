//! The expand-pip toggle system: a press on a member row's `+`/`-` pip flips its
//! [`PipExpanded`] state, re-glyphs it in place, AND drives the matching [`MemberStatPanel`]'s
//! GTW-416 accordion open / closed so the per-member stat table lerps (GTW-428 C1).
//!
//! The pip is the toggle CONTROL (the ticket's "wire the existing ExpandPip/PipExpanded to drive
//! the accordion target"): it does not own the lerp. On a press it sets the panel's
//! [`AccordionAnim`](gdtf_ui::AccordionAnim) to its [`toggled`](gdtf_ui::AccordionAnim::toggled)
//! direction; the SHARED `drive_accordions` system (registered by `UiPlugin`) then advances the
//! panel's [`AccordionProgress`](gdtf_ui::AccordionProgress) and writes the lerped height each
//! frame — pushing the rows below it down as it grows. The panel is matched to the pip by
//! [`MemberRowIndex`], not the entity tree, so it stays correct across re-keys / deletes (C5).

use bevy::{prelude::*, ui::Interaction};
use gdtf_ui::AccordionAnim;

use crate::states::running::gang_editor::components::{
    ExpandPip, MemberRowIndex, MemberStatPanel, PipExpanded,
};

/// The glyph the pip shows when collapsed (the default).
const COLLAPSED_GLYPH: &str = "+";

/// The glyph the pip shows when expanded.
const EXPANDED_GLYPH: &str = "-";

/// The press-edge query filter [`toggle_expand_pip`] reads — an [`ExpandPip`] whose
/// [`Interaction`] changed this frame. Named to keep the system signature under clippy's
/// `type_complexity` gate.
type PressedPip = (Changed<Interaction>, With<ExpandPip>);

/// Flips a member row's [`PipExpanded`] + re-glyphs the pip + drives its [`MemberStatPanel`]
/// accordion open / closed on a press (GTW-428 C1).
///
/// Reads `Changed<Interaction> == Pressed` on the [`ExpandPip`] buttons (one toggle per press edge
/// — the menu-action precedent). On a press it (1) flips the pip's [`PipExpanded`], (2) rewrites
/// the pip's own [`Text`] to `-` (expanded) / `+` (collapsed) IN PLACE (the ui-mutate rule), and
/// (3) finds the [`MemberStatPanel`] carrying the SAME [`MemberRowIndex`] and sets its
/// [`AccordionAnim`](gdtf_ui::AccordionAnim) to its toggled direction so the shared
/// `drive_accordions` lerps the panel open / closed. Registered
/// `run_if(in_state(RunningState::DebugGangEditor))`.
///
/// The two queries are disjoint (an [`ExpandPip`] is never a [`MemberStatPanel`]), and both read
/// [`MemberRowIndex`] only immutably, so there is no B0001 borrow conflict.
pub(in crate::states::running::gang_editor) fn toggle_expand_pip(
    mut pips: Query<(&Interaction, &MemberRowIndex, &mut PipExpanded, &mut Text), PressedPip>,
    mut panels: Query<(&MemberRowIndex, &mut AccordionAnim), With<MemberStatPanel>>,
) {
    for (interaction, pip_index, mut expanded, mut text) in &mut pips {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let now_expanded = expanded.toggle();
        let glyph = if now_expanded {
            EXPANDED_GLYPH
        } else {
            COLLAPSED_GLYPH
        };
        if text.0 != glyph {
            glyph.clone_into(&mut text.0);
        }
        // Drive the matching member's stat panel accordion (C1): toggle its animation direction so
        // the shared `drive_accordions` lerps its height open / closed.
        for (panel_index, mut anim) in &mut panels {
            if **panel_index == **pip_index {
                *anim = anim.toggled();
            }
        }
    }
}
