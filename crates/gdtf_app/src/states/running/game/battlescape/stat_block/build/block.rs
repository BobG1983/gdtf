//! The stat-block assembler: [`spawn_stat_block`] builds the root column (portrait,
//! name, faction, stance, bars, pips, lists) and stamps the widget-handle refs, plus
//! the shared themed text line. Split out of the monolithic `build.rs` (GTW-583); the
//! builder rationale lives on the parent `build` module.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Node, Val},
};
use gdtf_battle_presenter::TopDownAtlases;
use gdtf_ui::{
    FillFraction, FilledPips, spawn_pips, spawn_progress_bar,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::{
    bars::{spawn_bar_group, spawn_bar_label},
    lists::{spawn_injury_list, spawn_wound_list},
};
use crate::states::running::game::battlescape::stat_block::{
    colors::{HP_LOST, HP_REMAINING, TU_LOST, TU_REMAINING, WOUNDS_LOST, WOUNDS_REMAINING},
    components::{
        StatBlockRefs, StatFaction, StatHpBar, StatHpLabel, StatName, StatStance, StatTuBar,
        StatTuLabel, StatWoundsPips,
    },
    portrait::spawn_portrait,
    update::MAX_WOUND_PIPS,
};

/// The vertical gap between stat-block rows, as a fraction of the viewport HEIGHT
/// ([`Val::Vh`](bevy::ui::Val::Vh)).
///
/// A `const`, layout plumbing fed straight to a [`Node`] (the `CELL_PX`-class carve-out). A
/// vertical gap, so the unit is `Vh`; 0.55556 vh is 4 px at the 720-tall reference window
/// (`ui-responsive-not-px`).
pub(super) const ROW_GAP_VH: f32 = 0.55556;

/// Spawns one ganger stat block and returns its root [`Entity`].
///
/// Builds, as children of a vertical-column root: the portrait
/// [`ImageNode`](bevy::ui::widget::ImageNode) (face 0 placeholder until the update sets
/// it), the name title, the faction line, the stance line, a TU
/// [`ProgressBar`](gdtf_ui::spawn_progress_bar) (empty), an HP `ProgressBar` (empty), a
/// Wounds [`Pips`](gdtf_ui::spawn_pips) row (`WoundsMax`-derived total is set on update —
/// spawned with a default pip count), and a hidden wound-name list of [`WOUND_LINE_POOL`](super::lists::WOUND_LINE_POOL)
/// pooled lines. Stamps the [`StatBlockRefs`] handle on the root so the per-update mutate
/// finds every widget by its stored id.
///
/// Returns the root unparented; the caller (a panel's spawn system) parents it under its
/// own panel root. The portrait reads the presenter's
/// [`SheetRole::Portraits`](gdtf_battle_presenter::SheetRole) atlas off [`TopDownAtlases`];
/// if that sheet is not loaded the portrait node is spawned WITHOUT an image (an empty
/// node — the panel still renders, the face simply appears once the atlas is present, the
/// update sets the index regardless).
pub(in crate::states::running::game::battlescape) fn spawn_stat_block(
    commands: &mut Commands,
    theme: &GdtfTheme,
    atlases: Option<&TopDownAtlases>,
) -> Entity {
    let portrait = spawn_portrait(commands, atlases);
    let name = spawn_text(commands, theme, StatName, "");
    let faction = spawn_text(commands, theme, StatFaction, "");
    let stance = spawn_text(commands, theme, StatStance, "");
    // The bars/pips spawn empty (the update fills them from the ganger's components).
    // Each bar + its cur/max number are wrapped in a small column: the number sits ABOVE
    // the bar on the DARK panel background (not overlaid on the bright bar) so it reads
    // with contrast and never overflows the thin track (GTW-310 rework).
    let tu_bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        TU_REMAINING,
        TU_LOST,
        StatTuBar,
    );
    let tu_label = spawn_bar_label(commands, theme, StatTuLabel);
    let tu_group = spawn_bar_group(commands, tu_label, tu_bar);
    let hp_bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        HP_REMAINING,
        HP_LOST,
        StatHpBar,
    );
    let hp_label = spawn_bar_label(commands, theme, StatHpLabel);
    let hp_group = spawn_bar_group(commands, hp_label, hp_bar);
    // A FIXED pool of pips — the update shows the first `WoundsMax` of them (filled per
    // `Wounds`) and hides the rest, so the displayed pip count tracks the ganger's
    // `WoundsMax` without ever respawning the row ([[ui-mutate-not-respawn]]).
    let wounds = spawn_pips(
        commands,
        MAX_WOUND_PIPS,
        FilledPips::new(0),
        WOUNDS_REMAINING,
        WOUNDS_LOST,
        StatWoundsPips,
    );
    let wound_list = spawn_wound_list(commands, theme);
    let injury_list = spawn_injury_list(commands, theme);

    // GTW-322 — authored via `spawn_scene`. The root's only components are the
    // runtime-valued column `Node` (composed with `template_value`) and the
    // `StatBlockRefs` handle (a struct of live `Entity` ids — it has no meaningful
    // `Default`, so it is `.insert`ed after the scene is queued rather than seeded
    // into the reflection-free `bsn!` grammar; its components are disjoint from the
    // scene's so the same root + components result).
    let root_node = Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(ROW_GAP_VH),
        ..default()
    };
    let root = commands
        .spawn_scene(template_value(root_node))
        .insert(StatBlockRefs {
            portrait,
            name,
            faction,
            stance,
            tu_bar,
            tu_label,
            hp_bar,
            hp_label,
            wounds,
            wound_list,
            injury_list,
        })
        .id();
    commands.entity(root).add_children(&[
        portrait,
        name,
        faction,
        stance,
        tu_group,
        hp_group,
        wounds,
        wound_list,
        injury_list,
    ]);
    root
}

/// Spawns one themed body-text line carrying its `marker`, starting at `initial`.
///
/// A themed [`Text`](bevy::prelude::Text) with the body-text sub-theme's font/color and a
/// [`Themed(ThemeRole::Text)`](Themed) marker so `apply_theme` restyles it on a theme
/// change (the status-panel `spawn_line` idiom).
fn spawn_text(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    initial: &str,
) -> Entity {
    // GTW-322 — `Themed` + `Text::new` ride the `bsn!` macro inline; `UiTextColor` is a
    // simple tuple-component value; `TextFont` is NOT `Unpin`, so it rides the
    // `template(|_| ..)` closure escape hatch (the `spawn_button` caption precedent), and
    // the generic `marker` is `.insert`ed after the scene is queued.
    let text_color = *theme.text.text_color;
    // The macro's `Type::new(expr)` stores a DEFERRED constructor, so the seed string must
    // be owned (`'static`) — a borrowed `&str` would not outlive the deferred apply.
    let caption = initial.to_owned();
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            Themed::new(ThemeRole::Text)
            Text::new(caption)
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .insert(marker)
        .id()
}
