//! The pooled wound / injury name lists: fixed pools of hidden themed lines the
//! per-update shows/hides in place. Split out of the monolithic `build.rs` (GTW-583);
//! the builder rationale lives on the parent `build` module.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
};
use gdtf_ui::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::running::game::battlescape::stat_block::components::{
    StatInjuryLine, StatInjuryList, StatWoundLine, StatWoundList,
};

/// The maximum number of wound-name lines a stat block renders.
///
/// The wound-name list pre-spawns this many [`Text`] lines once and the update shows the
/// first N (one per inflicted wound) / hides the rest, so the list mutates in place
/// rather than spawning a line per wound ([[ui-mutate-not-respawn]]). A ganger with more
/// than this many wounds shows the first [`WOUND_LINE_POOL`] (a reasonable display cap; a
/// scrolling list is later polish). A `const`, not a domain newtype — a pool size fed to
/// a loop (`.claude/rules/no-bare-types.md` clause 4).
pub(super) const WOUND_LINE_POOL: usize = 8;

/// The maximum number of injury-name lines a stat block renders (GTW-439).
///
/// The injury-name list pre-spawns this many [`Text`] lines once and the update shows the
/// first N (one per inflicted injury) / hides the rest, so the list mutates in place rather
/// than spawning a line per injury ([[ui-mutate-not-respawn]]) — the [`WOUND_LINE_POOL`]
/// precedent. A ganger with more than this many injuries shows the first
/// [`INJURY_LINE_POOL`] (a reasonable display cap; a scrolling list is later polish). A
/// `const`, not a domain newtype — a pool size fed to a loop
/// (`.claude/rules/no-bare-types.md` clause 4).
pub(super) const INJURY_LINE_POOL: usize = 8;

/// Spawns the wound-name list container (hidden) with [`WOUND_LINE_POOL`] pooled,
/// hidden line children, and returns the container [`Entity`].
///
/// The container is a vertical column marked [`StatWoundList`]; each pooled line is a
/// themed [`Text`](bevy::prelude::Text) marked [`StatWoundLine`], started empty and
/// [`Visibility::Hidden`]. The update shows the first N lines (one per wound), sets their
/// content, hides the rest, and shows/hides the container — all mutate-in-place
/// ([[ui-mutate-not-respawn]]).
pub(super) fn spawn_wound_list(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    // GTW-322 — the container authors `StatWoundList` via `bsn!`; its runtime-valued
    // `Node` + `Visibility::Hidden` ride `template_value`. Each pooled line authors
    // `StatWoundLine` + `Themed` + `Text::new` + `UiTextColor` inline, with `TextFont`
    // (not `Unpin`) on the `template(|_| ..)` closure and `Visibility::Hidden` composed
    // with `template_value`.
    let container_node = Node {
        flex_direction: FlexDirection::Column,
        ..default()
    };
    let container = commands
        .spawn_scene((
            bsn! { StatWoundList },
            template_value(container_node),
            // Hidden until the ganger has ≥1 inflicted wound.
            template_value(Visibility::Hidden),
        ))
        .id();
    let text_color = *theme.text.text_color;
    let lines: Vec<Entity> = (0..WOUND_LINE_POOL)
        .map(|_| {
            let text_font = TextFont {
                font: theme.text.font.clone().into(),
                font_size: FontSize::Px(*theme.text.font_size_pt),
                ..default()
            };
            commands
                .spawn_scene((
                    bsn! {
                        StatWoundLine
                        Themed::new(ThemeRole::Text)
                        Text::new("")
                        UiTextColor(text_color)
                        template(move |_| Ok(text_font.clone()))
                    },
                    template_value(Visibility::Hidden),
                ))
                .id()
        })
        .collect();
    commands.entity(container).add_children(&lines);
    container
}

/// Spawns the injury-name list container (hidden) with [`INJURY_LINE_POOL`] pooled, hidden
/// line children, and returns the container [`Entity`] (GTW-439).
///
/// The structural twin of [`spawn_wound_list`]: a vertical column marked [`StatInjuryList`];
/// each pooled line is a themed [`Text`](bevy::prelude::Text) marked [`StatInjuryLine`],
/// started empty and [`Visibility::Hidden`]. The update shows the first N lines (one per
/// inflicted injury) with each [`GainedInjury`](gdtf_battle_sim::injuries::GainedInjury)'s authored
/// `inspect_text`, hides the rest, and shows/hides the container — all mutate-in-place
/// ([[ui-mutate-not-respawn]]). Driven by the DURABLE
/// [`InflictedInjuries`](gdtf_battle_sim::injuries::InflictedInjuries) ledger, so the list persists
/// while the ganger is inspected/selected (the transient FCT flash is a separate concern).
pub(super) fn spawn_injury_list(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    // GTW-322 — the container authors `StatInjuryList` via `bsn!`; its runtime-valued `Node`
    // + `Visibility::Hidden` ride `template_value`. Each pooled line authors `StatInjuryLine`
    // + `Themed` + `Text::new` + `UiTextColor` inline, with `TextFont` (not `Unpin`) on the
    // `template(|_| ..)` closure and `Visibility::Hidden` composed with `template_value` (the
    // `spawn_wound_list` precedent).
    let container_node = Node {
        flex_direction: FlexDirection::Column,
        ..default()
    };
    let container = commands
        .spawn_scene((
            bsn! { StatInjuryList },
            template_value(container_node),
            // Hidden until the ganger has ≥1 inflicted injury.
            template_value(Visibility::Hidden),
        ))
        .id();
    let text_color = *theme.text.text_color;
    let lines: Vec<Entity> = (0..INJURY_LINE_POOL)
        .map(|_| {
            let text_font = TextFont {
                font: theme.text.font.clone().into(),
                font_size: FontSize::Px(*theme.text.font_size_pt),
                ..default()
            };
            commands
                .spawn_scene((
                    bsn! {
                        StatInjuryLine
                        Themed::new(ThemeRole::Text)
                        Text::new("")
                        UiTextColor(text_color)
                        template(move |_| Ok(text_font.clone()))
                    },
                    template_value(Visibility::Hidden),
                ))
                .id()
        })
        .collect();
    commands.entity(container).add_children(&lines);
    container
}
