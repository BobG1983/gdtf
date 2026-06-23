//! Builds the status-panel **stability readout** row (GTW-345): a "STAB" caption above an
//! empty [`ProgressBar`](gdtf_ui::spawn_progress_bar), returned as one row [`Entity`] the
//! status-panel spawn parents under its root.
//!
//! The widgets spawn at the EMPTY / zero state (an empty bar, a static caption); the
//! per-update [`update_stability_readout`](super::update::update_stability_readout) repaints
//! the bar fill by MUTATING the existing fill node ([[ui-mutate-not-respawn]]). The readout
//! is a SIBLING row under the status-panel root, NOT inside the shared
//! [`stat_block`](super::super::super::stat_block) (which the inspect panel reuses for a
//! hovered target that is not necessarily a shooter).

use bevy::{
    color::Color,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Node, Val},
};
use gdtf_ui::{
    FillFraction, spawn_progress_bar,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::running::game::battlescape::status_panel::stability_readout::components::StabilityBar;

/// The static caption shown above the stability bar — names the readout for the player.
///
/// A `const`, copy fed straight to a [`Text`] (the stat-block `NO_TARGET` precedent — UI
/// copy, not a domain value, `.claude/rules/no-bare-types.md` clause 4).
const STAB_CAPTION: &str = "STAB";

/// The stability bar's **remaining** (filled) color — green, matching the
/// `stat-display-bars-pips` convention (a positive stat reads green, like the HP bar's
/// remaining fill).
///
/// Plain `Color` plumbing fed straight to the widget (the stat-block colors precedent — the
/// `gdtf_ui` `ProgressBar` takes any two colors; `.claude/rules/no-bare-types.md` clause 4).
const STAB_REMAINING: Color = Color::srgb(0.30, 0.78, 0.36);

/// The stability bar's **lost** (track) color — a dark panel-bg, matching the TU / Wounds
/// track color so the readout sits cleanly on the dark panel.
const STAB_LOST: Color = Color::srgb(0.12, 0.14, 0.16);

/// The font size of the "STAB" caption, in points — matches the bar-label size used by the
/// TU / HP `cur/max` numbers so the readout reads consistently with the bars above it.
///
/// `font_size` in points is the ONE permitted bare-`f32` exception to `ui-responsive-not-px`
/// (a typographic size, not a layout dimension); a `const` so the size lives in one place.
const STAB_CAPTION_FONT_PT: f32 = 12.0;

/// The vertical gap between the caption and its bar, as a fraction of the viewport HEIGHT
/// ([`Val::Vh`](bevy::ui::Val::Vh)) — keeps the caption off the bar's top edge (the
/// stat-block `ROW_GAP_VH` precedent; layout plumbing fed straight to a [`Node`]).
const ROW_GAP_VH: f32 = 0.55556;

/// Spawns the stability-readout row (a "STAB" caption ABOVE an empty
/// [`ProgressBar`](gdtf_ui::spawn_progress_bar)) and returns the row [`Entity`].
///
/// The bar is built at an EMPTY fill (the update fills it from the selected shooter's
/// steadiness) and tagged [`StabilityBar`] so the per-update mutate finds it. The caption is
/// a static [`Themed(ThemeRole::Text)`](Themed) line (so `apply_theme` restyles it on a theme
/// change) sitting above the bar in a tight vertical column, mirroring the stat-block's
/// `spawn_bar_group` layout (number/caption on the DARK panel background, bar below). Returns
/// the row unparented; the status-panel spawn parents it under its panel root.
pub(in crate::states::running::game::battlescape::status_panel) fn spawn_stability_readout(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    let caption = spawn_caption(commands, theme);
    // Empty fill at spawn — the update repaints it from the shooter's steadiness.
    let bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        STAB_REMAINING,
        STAB_LOST,
        StabilityBar,
    );

    // A tight vertical column: caption ON TOP (on the dark panel background), bar below.
    let row_node = Node {
        flex_direction: FlexDirection::Column,
        width: Val::Percent(100.0),
        row_gap: Val::Vh(ROW_GAP_VH),
        ..default()
    };
    let row = commands.spawn_scene(template_value(row_node)).id();
    commands.entity(row).add_children(&[caption, bar]);
    row
}

/// Spawns the static "STAB" caption [`Text`] line and returns its [`Entity`].
///
/// A [`Themed(ThemeRole::Text)`](Themed) line at the legible [`STAB_CAPTION_FONT_PT`], the
/// SAME light theme color the name / faction / stance lines use (reads cleanly on the dark
/// panel). The caption is static — it is never mutated by the update.
fn spawn_caption(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    // GTW-322 — `Themed` + `Text::new` + `UiTextColor` ride the `bsn!` macro inline;
    // `TextFont` (not `Unpin`) rides the `template(|_| ..)` closure escape hatch (the
    // stat-block `spawn_bar_label` precedent).
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(STAB_CAPTION_FONT_PT),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            Themed::new(ThemeRole::Text)
            Text::new(STAB_CAPTION)
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .id()
}
