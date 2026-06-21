//! Spawns + despawns the battlescape inspect panel (GTW-274 / GTW-295).
//!
//! [`spawn_inspect_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a themed
//! [`gdtf_ui`] panel on the GTW-120 UI camera as an ABSOLUTE, fixed-% (`Val::Vw`/`Val::Vh`)
//! overlay anchored TOP-RIGHT — hovering OVER the map, contributing NOTHING to the viewport
//! inset (only the bottom bar reduces the map). It holds TWO mutually
//! exclusive sub-blocks: the shared ganger [`stat_block`](super::super::super::stat_block)
//! (marked [`InspectStatBlockHost`]) and a small OBJECT stat block (marked
//! [`InspectObjectBlock`]) — a title, a labeled Integrity [`ProgressBar`](gdtf_ui::spawn_progress_bar)
//! ([`InspectObjectBar`]), and labeled Hardness / Protection / Height-band lines, read from a
//! hovered cover's [`CoverEntry`](gdtf_battle_sim::CoverEntry).
//!
//! Each sub-block is HIDDEN via [`Display::None`] (removed from layout, GTW-295) so the
//! panel sizes to the VISIBLE block only — a hidden tall ganger block no longer balloons the
//! panel on a small object hover. The whole panel root starts `Visibility::Hidden`;
//! [`update_inspect_panel`](super::update::update_inspect_panel) toggles it and its sub-blocks from
//! the EFFECTIVE [`InspectTarget`](gdtf_battle_input::InspectTarget) every battle frame, mutating in place.
//!
//! [`despawn_inspect_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the panel by its [`InspectPanelRoot`] marker — battle-scoped, mirroring the status
//! panel.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Display, Node, OverflowAxis, Val},
};
use gdtf_battle_presenter::TopDownAtlases;
use gdtf_ui::{FillFraction, spawn_panel, spawn_progress_bar, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    inspect_panel::components::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectIntegrity, InspectObjectProtection, InspectObjectText, InspectPanelRoot,
        InspectStatBlockHost,
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

/// The inspect panel's **fixed** width as a fraction of the viewport WIDTH
/// ([`Val::Vw`](bevy::ui::Val::Vw)). A relative unit (`ui-responsive-not-px`), and *fixed* so
/// the map area never pops/shifts left-right when the panel's contents change — the same
/// ganger vs. object block always occupies this one width (item 7). The panel is an absolute
/// overlay (item 3), so this width contributes NOTHING to the world-map viewport inset.
const PANEL_WIDTH_VW: f32 = 18.0;

/// The inspect panel's **maximum** height as a fraction of the viewport HEIGHT
/// ([`Val::Vh`](bevy::ui::Val::Vh)). The panel height still tracks its visible block's content
/// (`height: auto`), but is CAPPED here so a tall ganger block can never overrun the window;
/// the cap is relative (`ui-responsive-not-px`). Excess content is clipped (overflow hidden)
/// rather than overflowing onto the map.
const PANEL_MAX_HEIGHT_VH: f32 = 45.0;

/// Builds the themed inspect panel on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] (`Option<Res<…>>`, no-op if absent, `bevy-traps.md` #1) +
/// the presenter's [`TopDownAtlases`] (for the portrait). Spawns the panel root via
/// [`spawn_panel`] as an ABSOLUTE, fixed-% (`Val::Vw`/`Val::Vh`) overlay anchored TOP-RIGHT
/// (contributing nothing to the viewport inset) and `Visibility::Hidden`, parents the shared stat
/// block (marked [`InspectStatBlockHost`]) and the OBJECT block (marked [`InspectObjectBlock`])
/// under it. BOTH sub-blocks start HIDDEN via [`Display::None`] (removed from layout, GTW-295)
/// so the panel sizes to the visible block only. The object block holds a title, a labeled
/// Integrity [`ProgressBar`](gdtf_ui::spawn_progress_bar), and labeled Hardness / Protection /
/// Height-band lines (AC3). Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::game::battlescape) fn spawn_inspect_panel(
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
        InspectPanelRoot,
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
    commands.entity(block).insert(InspectStatBlockHost);

    let object_block = spawn_object_block(&mut commands, &theme);

    commands.entity(root).add_children(&[block, object_block]);
}

/// Spawns the OBJECT stat block (GTW-295 AC3) and returns its container [`Entity`].
///
/// A vertical column ([`InspectObjectBlock`], hidden via [`Display::None`] until an object
/// hover) holding, top to bottom: a TITLE line ([`InspectObjectText`], e.g. "Cover"), a labeled
/// Integrity row (an "Integrity" label + the [`InspectObjectBar`] [`ProgressBar`]), and labeled
/// Hardness ([`InspectObjectHardness`]) / Protection ([`InspectObjectProtection`]) / Height-band
/// ([`InspectObjectHeight`]) [`Text`] lines — a readable block comparable to the ganger block.
/// The widgets seed empty; the update fills them from the hovered [`CoverEntry`](gdtf_battle_sim::CoverEntry)
/// in place ([[ui-mutate-not-respawn]]).
fn spawn_object_block(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let title = spawn_line(commands, theme, InspectObjectText, "");
    let integrity_label = spawn_line(commands, theme, InspectObjectIntegrity, "Integrity");
    let bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        INTEGRITY_REMAINING,
        INTEGRITY_LOST,
        InspectObjectBar,
    );
    let hardness = spawn_line(commands, theme, InspectObjectHardness, "");
    let protection = spawn_line(commands, theme, InspectObjectProtection, "");
    let height = spawn_line(commands, theme, InspectObjectHeight, "");

    // GTW-322 — the `InspectObjectBlock` marker rides the `bsn!` macro inline; the
    // runtime-valued column `Node` is composed with `template_value`.
    let block_node = Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(ROW_GAP_VH),
        // Hidden via Display::None (removed from layout) until an object hover; its
        // VISIBILITY inherits from the root (the update flips only its `display`).
        display: Display::None,
        ..default()
    };
    let block = commands
        .spawn_scene((bsn! { InspectObjectBlock }, template_value(block_node)))
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
    // GTW-322 — `Text::new` + `UiTextColor` ride the `bsn!` macro inline; `TextFont` (not
    // `Unpin`) rides the `template(|_| ..)` closure; the generic `marker` is `.insert`ed
    // after the scene is queued. The seed string is owned (`'static`) for the deferred apply.
    let text_color = *theme.text.text_color;
    let caption = initial.to_owned();
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            Text::new(caption)
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .insert(marker)
        .id()
}

/// Despawns the inspect panel on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`InspectPanelRoot`] entity (and its sub-blocks) — battle-scoped
/// lifecycle. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::game::battlescape) fn despawn_inspect_panel(
    mut commands: Commands,
    panels: Query<Entity, With<InspectPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
