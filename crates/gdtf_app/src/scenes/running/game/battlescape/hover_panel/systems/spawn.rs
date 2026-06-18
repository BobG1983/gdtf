//! Spawns + despawns the battlescape hover-inspect panel (GTW-274 / GTW-295).
//!
//! [`spawn_hover_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a themed
//! [`gdtf_ui`] panel on the GTW-120 UI camera as an ABSOLUTE, fixed-% (`Val::Vw`/`Val::Vh`)
//! overlay anchored TOP-RIGHT — hovering OVER the map, contributing NOTHING to the viewport
//! inset (only the bottom bar reduces the map). It holds TWO mutually
//! exclusive sub-blocks: the shared ganger [`stat_block`](super::super::super::stat_block)
//! (marked [`HoverStatBlockHost`]) and a small OBJECT stat block (marked
//! [`HoverObjectBlock`]) — a title, a labeled Integrity [`ProgressBar`](gdtf_ui::spawn_progress_bar)
//! ([`HoverObjectBar`]), and labeled Hardness / Protection / Height-band lines, read from a
//! hovered cover's [`CoverEntry`](gdtf_battle_sim::CoverEntry).
//!
//! Each sub-block is HIDDEN via [`Display::None`] (removed from layout, GTW-295) so the
//! panel sizes to the VISIBLE block only — a hidden tall ganger block no longer balloons the
//! panel on a small object hover. The whole panel root starts `Visibility::Hidden`;
//! [`update_hover_panel`](super::update::update_hover_panel) toggles it and its sub-blocks from
//! the [`HoveredCell`](gdtf_battle_input::HoveredCell) every battle frame, mutating in place.
//!
//! [`despawn_hover_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the panel by its [`HoverPanelRoot`] marker — battle-scoped, mirroring the status
//! panel.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{Display, Node, OverflowAxis, Val},
};
use gdtf_battle_presenter::TopDownAtlases;
use gdtf_ui::{FillFraction, spawn_panel, spawn_progress_bar, theme::GdtfTheme};

use crate::scenes::running::game::battlescape::{
    hover_panel::components::{
        HoverObjectBar, HoverObjectBlock, HoverObjectHardness, HoverObjectHeight,
        HoverObjectIntegrity, HoverObjectProtection, HoverObjectText, HoverPanelRoot,
        HoverStatBlockHost,
    },
    stat_block::spawn_stat_block,
};

/// The integrity bar's **remaining** (filled) color — a structural grey-green.
const INTEGRITY_REMAINING: Color = Color::srgb(0.55, 0.65, 0.45);

/// The integrity bar's **lost** (track) color — a dark panel-bg.
const INTEGRITY_LOST: Color = Color::srgb(0.12, 0.14, 0.16);

/// The vertical gap between object-block rows, as a fraction of the viewport HEIGHT
/// ([`Val::Vh`](bevy::ui::Val::Vh)) — the stat-block `ROW_GAP_VH` precedent, a small fixed
/// hairline-class spacing. A vertical gap, so the unit is `Vh`; 0.55556 vh is 4 px at the
/// 720-tall reference window (`ui-responsive-not-px`).
const ROW_GAP_VH: f32 = 0.55556;

/// The hover panel's **fixed** width as a fraction of the viewport WIDTH
/// ([`Val::Vw`](bevy::ui::Val::Vw)). A relative unit (`ui-responsive-not-px`), and *fixed* so
/// the map area never pops/shifts left-right when the panel's contents change — the same
/// ganger vs. object block always occupies this one width (item 7). The panel is an absolute
/// overlay (item 3), so this width contributes NOTHING to the world-map viewport inset.
const PANEL_WIDTH_VW: f32 = 18.0;

/// The hover panel's **maximum** height as a fraction of the viewport HEIGHT
/// ([`Val::Vh`](bevy::ui::Val::Vh)). The panel height still tracks its visible block's content
/// (`height: auto`), but is CAPPED here so a tall ganger block can never overrun the window;
/// the cap is relative (`ui-responsive-not-px`). Excess content is clipped (overflow hidden)
/// rather than overflowing onto the map.
const PANEL_MAX_HEIGHT_VH: f32 = 45.0;

/// Builds the themed hover panel on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] (`Option<Res<…>>`, no-op if absent, `bevy-traps.md` #1) +
/// the presenter's [`TopDownAtlases`] (for the portrait). Spawns the panel root via
/// [`spawn_panel`] as an ABSOLUTE, fixed-% (`Val::Vw`/`Val::Vh`) overlay anchored TOP-RIGHT
/// (contributing nothing to the viewport inset) and `Visibility::Hidden`, parents the shared stat
/// block (marked [`HoverStatBlockHost`]) and the OBJECT block (marked [`HoverObjectBlock`])
/// under it. BOTH sub-blocks start HIDDEN via [`Display::None`] (removed from layout, GTW-295)
/// so the panel sizes to the visible block only. The object block holds a title, a labeled
/// Integrity [`ProgressBar`](gdtf_ui::spawn_progress_bar), and labeled Hardness / Protection /
/// Height-band lines (AC3). Param-only (`bevy-traps.md` #7).
pub(in crate::scenes::running::game::battlescape) fn spawn_hover_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    atlases: Option<Res<TopDownAtlases>>,
) {
    let Some(theme) = theme else {
        return;
    };

    // ROOT: an ABSOLUTE, fixed-% overlay anchored TOP-RIGHT, hovering OVER the map (item 3).
    // It is a UI node on the over-map UI camera, positioned `Absolute` so it is removed from
    // any layout flow and contributes NOTHING to the world-map viewport inset — only the bottom
    // bar reduces the map (item 4). Its width is a FIXED fraction of the viewport
    // (`Val::Vw`, `ui-responsive-not-px`) so the map never pops/shifts when the panel's
    // contents change (item 7); its height tracks the visible block but is CAPPED in `Val::Vh`
    // and clips overflow so a tall ganger block can never overrun the map. Replacing the
    // builder's `Node` is fine: `apply_theme` re-derives the panel's border / radius / padding
    // from this node every run (it clones the node and overrides only those), so the framed
    // look is preserved while the size + position here stay authoritative.
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        HoverPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Vh(0.0),
            right: Val::Vw(0.0),
            width: Val::Vw(PANEL_WIDTH_VW),
            height: Val::Auto,
            max_height: Val::Vh(PANEL_MAX_HEIGHT_VH),
            flex_direction: FlexDirection::Column,
            overflow: Overflow {
                x: OverflowAxis::Hidden,
                y: OverflowAxis::Hidden,
            },
            ..default()
        },
        Visibility::Hidden,
    ));

    // The shared ganger stat block, marked as THIS panel's host. Its VISIBILITY is left to
    // INHERIT from the root (which starts `Visibility::Hidden`, so the host is hidden until the
    // panel shows); its hide/show is driven purely by its layout `Display` (Flex on a ganger
    // hover, None otherwise, GTW-295), so a hidden ganger block is REMOVED from layout and
    // never balloons the panel. Its layout `Node` (the stat-block column) is left as the
    // builder made it — the update flips only the `display`, never clobbering the column
    // layout. The update sets its display each frame; it starts at the builder default (Flex),
    // harmless since the root starts Hidden.
    let block = spawn_stat_block(&mut commands, &theme, atlases.as_deref());
    commands.entity(block).insert(HoverStatBlockHost);

    let object_block = spawn_object_block(&mut commands, &theme);

    commands.entity(root).add_children(&[block, object_block]);
}

/// Spawns the OBJECT stat block (GTW-295 AC3) and returns its container [`Entity`].
///
/// A vertical column ([`HoverObjectBlock`], hidden via [`Display::None`] until an object
/// hover) holding, top to bottom: a TITLE line ([`HoverObjectText`], e.g. "Cover"), a labeled
/// Integrity row (an "Integrity" label + the [`HoverObjectBar`] [`ProgressBar`]), and labeled
/// Hardness ([`HoverObjectHardness`]) / Protection ([`HoverObjectProtection`]) / Height-band
/// ([`HoverObjectHeight`]) [`Text`] lines — a readable block comparable to the ganger block.
/// The widgets seed empty; the update fills them from the hovered [`CoverEntry`](gdtf_battle_sim::CoverEntry)
/// in place ([[ui-mutate-not-respawn]]).
fn spawn_object_block(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let title = spawn_line(commands, theme, HoverObjectText, "");
    let integrity_label = spawn_line(commands, theme, HoverObjectIntegrity, "Integrity");
    let bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        INTEGRITY_REMAINING,
        INTEGRITY_LOST,
        HoverObjectBar,
    );
    let hardness = spawn_line(commands, theme, HoverObjectHardness, "");
    let protection = spawn_line(commands, theme, HoverObjectProtection, "");
    let height = spawn_line(commands, theme, HoverObjectHeight, "");

    let block = commands
        .spawn((
            HoverObjectBlock,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Vh(ROW_GAP_VH),
                // Hidden via Display::None (removed from layout) until an object hover; its
                // VISIBILITY inherits from the root (the update flips only its `display`).
                display: Display::None,
                ..default()
            },
        ))
        .id();
    commands.entity(block).add_children(&[
        title,
        integrity_label,
        bar,
        hardness,
        protection,
        height,
    ]);
    block
}

/// Spawns one themed body-text line carrying `marker`, started at `initial` (the stat-block
/// `spawn_text` idiom). The update mutates the text in place ([[ui-mutate-not-respawn]]).
fn spawn_line(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    initial: &str,
) -> Entity {
    commands
        .spawn((
            marker,
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
