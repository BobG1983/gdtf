//! Spawns the battlescape LOADING SCREEN on entry to Generation (GTW-419).
//!
//! [`spawn_loading_screen`] runs `OnEnter(BattleScapeState::Generation)` and builds the
//! full-viewport overlay shown WHILE the sim assembles the level + builds the battle (the brief
//! Generation phase, gated on the sim's `BattleReady` signal). It is a PRESENTER / view artifact
//! living in the running scene — it owns no combat rules and never touches the sim.
//!
//! The overlay is ONE opaque full-window backdrop node (carrying the [`LoadingScreenRoot`]
//! marker, a high [`GlobalZIndex`], and [`DespawnOnExit`]`(Generation)`) that CENTERS a themed
//! panel holding a loading label. Because the level / presenter render on the WORLD camera and
//! the UI renders on the higher UI camera, an opaque full-viewport UI node paints OVER whatever
//! the world camera has drawn — so no frame of an unbuilt / partial level reaches the player
//! (AC2 / `bevy-traps.md` #8: occlusion working FOR us). The root is removed exactly on the
//! transition to `AnimateIn` via [`DespawnOnExit`], so the loading screen is present for the
//! WHOLE Generation duration and gone the instant the assembled level appears.

use bevy::{
    color::Alpha,
    prelude::*,
    state::prelude::DespawnOnExit,
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{
        AlignItems, BackgroundColor, GlobalZIndex, JustifyContent, Node, PositionType, UiRect, Val,
    },
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::loading_screen::components::{
        LOADING_BOX_MIN_H_VH, LOADING_BOX_W_VW, LOADING_SCREEN_Z, LoadingScreenRoot,
    },
};

/// The caption shown on the loading screen while the battlefield is assembled.
const LOADING_CAPTION: &str = "GENERATING BATTLEFIELD…";

/// Builds the full-viewport loading screen on `OnEnter(BattleScapeState::Generation)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — in the running app the theme is resolved during `Load`, so it is
/// present by the time a battle starts; the bottom-bar / combat-log precedent). With the theme
/// present it spawns:
///
/// 1. A ROOT backdrop [`Node`] — [`PositionType::Absolute`], FULL window
///    ([`Val::Vw`](bevy::ui::Val)`(100.0)` × [`Val::Vh`](bevy::ui::Val)`(100.0)`),
///    centering its child — with an OPAQUE [`BackgroundColor`] (the theme backdrop fill forced
///    to alpha `1.0`) so it covers any partial level beneath it. It carries the
///    [`LoadingScreenRoot`] marker, [`GlobalZIndex`]`(`[`LOADING_SCREEN_Z`]`)` (above every HUD
///    band so nothing can paint over it — AC2), and [`DespawnOnExit`]`(Generation)` so it is
///    despawned exactly on the transition to `AnimateIn`.
/// 2. A centered themed [`spawn_panel`](gdtf_ui::spawn_panel) box (bordered, panel fill / radius
///    re-painted by `apply_theme`) at a responsive width / min-height, parented under the root.
/// 3. A loading-caption [`Text`] label parented under the box, drawn in the theme's TITLE
///    typography (its resolved font handle + size + color).
///
/// All sizing is window-relative (`Vw`/`Vh`) — no fixed `Px` except `font_size`
/// (`ui-responsive-not-px`). Param-only (`bevy-traps.md` #7): [`Commands`] + the theme read.
pub(in crate::states::running::game::battlescape::generation) fn spawn_loading_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed screen (the bottom-bar
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    // The backdrop fill is the theme's full-screen background color, forced OPAQUE so the
    // partial / unbuilt level beneath (on the world camera) cannot bleed through (AC2).
    let mut backdrop = *theme.background.color;
    backdrop.set_alpha(1.0);

    // 1. The ROOT: a full-window absolute backdrop that centers its child. Carries the marker,
    //    the high stacking z, and the Generation-scoped despawn.
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::ZERO,
                top: Val::ZERO,
                width: Val::Vw(100.0),
                height: Val::Vh(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(backdrop),
            GlobalZIndex(LOADING_SCREEN_Z),
            LoadingScreenRoot,
            DespawnOnExit(BattleScapeState::Generation),
        ))
        .id();

    // 2. The centered themed panel box (bordered, theme-painted), sized responsively.
    let box_entity = spawn_panel(&mut commands, &theme);
    commands.entity(box_entity).insert(Node {
        width: Val::Vw(LOADING_BOX_W_VW),
        min_height: Val::Vh(LOADING_BOX_MIN_H_VH),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        // Breathing room around the caption so it never sits flush against the border.
        padding: UiRect::all(Val::Vw(*theme.panel.border_width * 2.0)),
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    });
    commands.entity(root).add_child(box_entity);

    // 3. The loading caption, drawn in the theme TITLE typography. The runtime-valued `TextFont`
    //    (not `Unpin`) is built before spawn (the combat-log `spawn_log_line` precedent).
    let title_font = TextFont {
        font: theme.title.font.clone().into(),
        font_size: FontSize::Px(*theme.title.font_size_pt),
        ..default()
    };
    let label = commands
        .spawn((
            Text::new(LOADING_CAPTION),
            UiTextColor(*theme.title.text_color),
            title_font,
        ))
        .id();
    commands.entity(box_entity).add_child(label);
}
