//! The expand-pip toggle system: a press on a member row's `+`/`-` pip flips its
//! [`PipExpanded`] state and re-glyphs it in place (GTW-425 C1).
//!
//! SCOPE: the pip is the toggle CONTROL only — GTW-425 renders the pip + its toggle state /
//! marker. The EXPANDED per-member stat table the pip would gate is GTW-428 and is OUT OF SCOPE
//! here (inline editing happens WITHOUT expanding). So this system only flips the state and the
//! glyph; it spawns no panel.

use bevy::{prelude::*, ui::Interaction};

use crate::states::running::editor::components::{ExpandPip, PipExpanded};

/// The glyph the pip shows when collapsed (the default).
const COLLAPSED_GLYPH: &str = "+";

/// The glyph the pip shows when expanded.
const EXPANDED_GLYPH: &str = "-";

/// The press-edge query filter [`toggle_expand_pip`] reads — an [`ExpandPip`] whose
/// [`Interaction`] changed this frame. Named to keep the system signature under clippy's
/// `type_complexity` gate.
type PressedPip = (Changed<Interaction>, With<ExpandPip>);

/// Flips a member row's [`PipExpanded`] and re-glyphs the pip on a press (GTW-425 C1).
///
/// Reads `Changed<Interaction> == Pressed` on the [`ExpandPip`] buttons (one toggle per press edge
/// — the menu-action precedent). On a press it flips the pip's [`PipExpanded`] and rewrites the
/// pip's own [`Text`] to `-` (expanded) or `+` (collapsed) IN PLACE (the ui-mutate rule). It does
/// NOT spawn or reveal an expanded panel — that is GTW-428. Registered
/// `run_if(in_state(RunningState::DebugEditor))`.
pub(in crate::states::running::editor) fn toggle_expand_pip(
    mut pips: Query<(&Interaction, &mut PipExpanded, &mut Text), PressedPip>,
) {
    for (interaction, mut expanded, mut text) in &mut pips {
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
    }
}
