//! The shared stat-block builder (GTW-278 / GTW-274): spawns ONE ganger stat block
//! (portrait, name, faction, stance, TU/HP bars, Wounds pips, wound-name list) and
//! returns its root [`Entity`] with the [`StatBlockRefs`] handle stamped on it.
//!
//! Both the status panel and the hover panel call [`spawn_stat_block`] to build their
//! block, then parent it under their own root — DRY: the render of a ganger's vitals
//! lives here once. The widgets are spawned at empty / zero values; the panel's
//! per-update [`update_stat_block`](super::update_stat_block) repaints them by MUTATING
//! the stored widget entities ([[ui-mutate-not-respawn]]).

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{Node, Val, widget::ImageNode},
};
use gdtf_battle_presenter::{SheetRole, TopDownAtlases};
use gdtf_ui::{
    FillFraction, FilledPips, spawn_pips, spawn_progress_bar,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::scenes::running::game::battlescape::stat_block::{
    colors::{HP_LOST, HP_REMAINING, TU_LOST, TU_REMAINING, WOUNDS_LOST, WOUNDS_REMAINING},
    components::{
        StatBlockRefs, StatFaction, StatHpBar, StatHpLabel, StatName, StatPortrait, StatStance,
        StatTuBar, StatTuLabel, StatWoundLine, StatWoundList, StatWoundsPips,
    },
    portrait::{PortraitIndex, portrait_node},
    update::MAX_WOUND_PIPS,
};

/// The font size of the `cur/max` numeric label sitting ABOVE its TU / HP bar, in points.
///
/// Matches the body-text size so the number reads cleanly on the DARK panel background
/// (the mockup pairing — `cur/max` shares the line/size of the caption it sits with), a
/// legible size, not a tiny overlay. `font_size` in points is the ONE permitted
/// bare-`f32` exception to `ui-responsive-not-px` (a typographic size, not a layout
/// dimension); a `const` so the label size lives in one place (GTW-310 rework).
const BAR_LABEL_FONT_PT: f32 = 12.0;

/// The maximum number of wound-name lines a stat block renders.
///
/// The wound-name list pre-spawns this many [`Text`] lines once and the update shows the
/// first N (one per inflicted wound) / hides the rest, so the list mutates in place
/// rather than spawning a line per wound ([[ui-mutate-not-respawn]]). A ganger with more
/// than this many wounds shows the first [`WOUND_LINE_POOL`] (a reasonable display cap; a
/// scrolling list is later polish). A `const`, not a domain newtype — a pool size fed to
/// a loop (`.claude/rules/no-bare-types.md` clause 4).
const WOUND_LINE_POOL: usize = 8;

/// The vertical gap between stat-block rows, as a fraction of the viewport HEIGHT
/// ([`Val::Vh`](bevy::ui::Val::Vh)).
///
/// A `const`, layout plumbing fed straight to a [`Node`] (the `CELL_PX`-class carve-out). A
/// vertical gap, so the unit is `Vh`; 0.55556 vh is 4 px at the 720-tall reference window
/// (`ui-responsive-not-px`).
const ROW_GAP_VH: f32 = 0.55556;

/// Spawns one ganger stat block and returns its root [`Entity`].
///
/// Builds, as children of a vertical-column root: the portrait
/// [`ImageNode`](bevy::ui::widget::ImageNode) (face 0 placeholder until the update sets
/// it), the name title, the faction line, the stance line, a TU
/// [`ProgressBar`](gdtf_ui::spawn_progress_bar) (empty), an HP `ProgressBar` (empty), a
/// Wounds [`Pips`](gdtf_ui::spawn_pips) row (`WoundsMax`-derived total is set on update —
/// spawned with a default pip count), and a hidden wound-name list of [`WOUND_LINE_POOL`]
/// pooled lines. Stamps the [`StatBlockRefs`] handle on the root so the per-update mutate
/// finds every widget by its stored id.
///
/// Returns the root unparented; the caller (a panel's spawn system) parents it under its
/// own panel root. The portrait reads the presenter's
/// [`SheetRole::Portraits`](gdtf_battle_presenter::SheetRole) atlas off [`TopDownAtlases`];
/// if that sheet is not loaded the portrait node is spawned WITHOUT an image (an empty
/// node — the panel still renders, the face simply appears once the atlas is present, the
/// update sets the index regardless).
pub(in crate::scenes::running::game::battlescape) fn spawn_stat_block(
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

    let root = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Vh(ROW_GAP_VH),
                ..default()
            },
            StatBlockRefs {
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
            },
        ))
        .id();
    commands.entity(root).add_children(&[
        portrait, name, faction, stance, tu_group, hp_group, wounds, wound_list,
    ]);
    root
}

/// Spawns the portrait [`ImageNode`](bevy::ui::widget::ImageNode) node at face 0.
///
/// Reads the presenter's [`SheetRole::Portraits`](gdtf_battle_presenter::SheetRole)
/// atlas off [`TopDownAtlases`] and builds the atlas-variant node
/// ([`portrait_node`](super::portrait::portrait_node)); if the sheet is not loaded it
/// spawns a bare marked node (no image) so the panel still builds — the update sets the
/// index on whatever node exists.
fn spawn_portrait(commands: &mut Commands, atlases: Option<&TopDownAtlases>) -> Entity {
    match atlases.and_then(|a| a.role(SheetRole::Portraits)) {
        Some(sheet) => commands
            .spawn(portrait_node(
                sheet.image.clone(),
                sheet.layout.clone(),
                PortraitIndex::for_name(None),
                StatPortrait,
            ))
            .id(),
        None => commands
            .spawn((
                ImageNode::default(),
                Node {
                    width: Val::ZERO,
                    height: Val::ZERO,
                    ..default()
                },
                StatPortrait,
            ))
            .id(),
    }
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
    commands
        .spawn((
            marker,
            Themed::new(ThemeRole::Text),
            Text::new(initial),
            TextFont {
                font: theme.text.font.clone(),
                font_size: *theme.text.font_size_pt,
                ..default()
            },
            UiTextColor(*theme.text.text_color),
        ))
        .id()
}

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
/// [`update_stat_block`](super::update_stat_block) writes `"cur/max"` in place
/// ([[ui-mutate-not-respawn]]).
fn spawn_bar_label(commands: &mut Commands, theme: &GdtfTheme, marker: impl Bundle) -> Entity {
    commands
        .spawn((
            marker,
            Themed::new(ThemeRole::Text),
            Text::new(""),
            TextFont {
                font: theme.text.font.clone(),
                font_size: BAR_LABEL_FONT_PT,
                ..default()
            },
            UiTextColor(*theme.text.text_color),
            TextLayout::new_with_justify(Justify::Right),
            // A full-width line so the right-justified number anchors to the bar's right
            // edge above it; the column wrapper gives it a row of its own (no overflow).
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
        ))
        .id()
}

/// Wraps a TU / HP `cur/max` `label` and its `bar` in a tight vertical column (label ON
/// TOP, bar below) and returns the wrapper [`Entity`].
///
/// This is the GTW-310 rework's contrast/overflow fix: the number rides on the DARK panel
/// background ABOVE the bar — off the bright bar fill — so it reads cleanly and has room.
/// The wrapper is a plain layout [`Node`] (no [`Themed`] marker, so `apply_theme` leaves
/// it alone) that consumes the bar's full width; the bar keeps its own
/// [`StatTuBar`](super::components::StatTuBar) /
/// [`StatHpBar`](super::components::StatHpBar) marker + its
/// [`ProgressBarFill`](gdtf_ui::ProgressBarFill) child untouched, so the per-update fill
/// query still finds it by stored id. A small `row_gap` keeps the number off the bar's top
/// edge.
fn spawn_bar_group(commands: &mut Commands, label: Entity, bar: Entity) -> Entity {
    let group = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(100.0),
            row_gap: Val::Vh(ROW_GAP_VH),
            ..default()
        })
        .id();
    commands.entity(group).add_children(&[label, bar]);
    group
}

/// Spawns the wound-name list container (hidden) with [`WOUND_LINE_POOL`] pooled,
/// hidden line children, and returns the container [`Entity`].
///
/// The container is a vertical column marked [`StatWoundList`]; each pooled line is a
/// themed [`Text`](bevy::prelude::Text) marked [`StatWoundLine`], started empty and
/// [`Visibility::Hidden`]. The update shows the first N lines (one per wound), sets their
/// content, hides the rest, and shows/hides the container — all mutate-in-place
/// ([[ui-mutate-not-respawn]]).
fn spawn_wound_list(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let container = commands
        .spawn((
            StatWoundList,
            Node {
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // Hidden until the ganger has ≥1 inflicted wound.
            Visibility::Hidden,
        ))
        .id();
    let lines: Vec<Entity> = (0..WOUND_LINE_POOL)
        .map(|_| {
            commands
                .spawn((
                    StatWoundLine,
                    Themed::new(ThemeRole::Text),
                    Text::new(""),
                    TextFont {
                        font: theme.text.font.clone(),
                        font_size: *theme.text.font_size_pt,
                        ..default()
                    },
                    UiTextColor(*theme.text.text_color),
                    Visibility::Hidden,
                ))
                .id()
        })
        .collect();
    commands.entity(container).add_children(&lines);
    container
}
