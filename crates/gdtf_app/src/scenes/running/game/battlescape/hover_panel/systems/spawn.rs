//! Spawns + despawns the battlescape hover-inspect panel (GTW-274).
//!
//! [`spawn_hover_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a themed
//! [`gdtf_ui`] panel on the GTW-120 UI camera anchored TOP-RIGHT, holding TWO mutually
//! exclusive sub-blocks: the shared ganger [`stat_block`](super::super::super::stat_block)
//! (marked [`HoverStatBlockHost`], hidden until a ganger is hovered) and a small object block
//! (marked [`HoverObjectBlock`] — a name/hardness `Text` + an integrity `ProgressBar`, hidden
//! until a non-floor object is hovered). The whole panel root starts `Visibility::Hidden`;
//! [`update_hover_panel`](super::update::update_hover_panel) toggles it and its sub-blocks from
//! the [`HoveredCell`](gdtf_battle_input::HoveredCell) every battle frame, mutating in place.
//!
//! [`despawn_hover_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the panel by its [`HoverPanelRoot`] marker — battle-scoped, mirroring the status
//! panel.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{Node, Val},
};
use gdtf_battle_presenter::TopDownAtlases;
use gdtf_ui::{FillFraction, spawn_panel, spawn_progress_bar, theme::GdtfTheme};

use crate::scenes::running::game::battlescape::{
    hover_panel::components::{
        HoverObjectBar, HoverObjectBlock, HoverObjectText, HoverPanelRoot, HoverStatBlockHost,
    },
    stat_block::spawn_stat_block,
};

/// The integrity bar's **remaining** (filled) color — a structural grey-green.
const INTEGRITY_REMAINING: Color = Color::srgb(0.55, 0.65, 0.45);

/// The integrity bar's **lost** (track) color — a dark panel-bg.
const INTEGRITY_LOST: Color = Color::srgb(0.12, 0.14, 0.16);

/// Builds the themed hover panel on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] (`Option<Res<…>>`, no-op if absent, `bevy-traps.md` #1) +
/// the presenter's [`TopDownAtlases`] (for the portrait). Spawns the panel root via
/// [`spawn_panel`] anchored TOP-RIGHT and `Visibility::Hidden`, parents the shared stat
/// block (marked [`HoverStatBlockHost`], hidden) and the object block (marked
/// [`HoverObjectBlock`], hidden) under it. Param-only (`bevy-traps.md` #7).
pub(in crate::scenes::running::game::battlescape) fn spawn_hover_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    atlases: Option<Res<TopDownAtlases>>,
) {
    let Some(theme) = theme else {
        return;
    };

    // ROOT: top-right anchored, hidden until something is hovered.
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        HoverPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            right: Val::Px(0.0),
            ..default()
        },
        Visibility::Hidden,
    ));

    // The shared ganger stat block, marked as THIS panel's host, hidden until a ganger hover.
    let block = spawn_stat_block(&mut commands, &theme, atlases.as_deref());
    commands
        .entity(block)
        .insert((HoverStatBlockHost, Visibility::Hidden));

    // The object block: a name/hardness line + an integrity bar, hidden until an object hover.
    let object_text = commands
        .spawn((
            HoverObjectText,
            Text::new(""),
            TextFont {
                font: theme.text.font.clone(),
                font_size: *theme.text.font_size_pt,
                ..default()
            },
            UiTextColor(*theme.text.text_color),
        ))
        .id();
    let object_bar = spawn_progress_bar(
        &mut commands,
        FillFraction::new(0.0),
        INTEGRITY_REMAINING,
        INTEGRITY_LOST,
        HoverObjectBar,
    );
    let object_block = commands
        .spawn((
            HoverObjectBlock,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            Visibility::Hidden,
        ))
        .id();
    commands
        .entity(object_block)
        .add_children(&[object_text, object_bar]);

    commands.entity(root).add_children(&[block, object_block]);
}

/// Despawns the hover panel on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`HoverPanelRoot`] entity (and its sub-blocks) — battle-scoped
/// lifecycle. Param-only (`bevy-traps.md` #7).
pub(in crate::scenes::running::game::battlescape) fn despawn_hover_panel(
    mut commands: Commands,
    panels: Query<Entity, With<HoverPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
