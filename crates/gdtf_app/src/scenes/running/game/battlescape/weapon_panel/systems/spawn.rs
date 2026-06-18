//! Spawns + despawns the battlescape weapon panel (GTW-275 / GTW-295, bottom-left).
//!
//! [`spawn_weapon_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a
//! themed [`gdtf_ui`] panel on the GTW-120 UI camera: a [`spawn_panel`] box
//! ([`WeaponPanelRoot`]) anchored BOTTOM-LEFT, laid out HORIZONTAL-FIRST to the mockup
//! (`assets/ui_mockups/battlescape_mockup.png` bottom-left cluster). The
//! [`WeaponContent`] column holds, top to bottom: a graphic ROW (a WIDE landscape weapon
//! graphic PLACEHOLDER beside the throwable placeholder slots), then an info column with
//! the weapon name, the magazine `"cur/max"` text, and a LIVE [`ReloadButton`].
//!
//! Sizing is RESPONSIVE (GTW-295): the panel + content carry window-relative
//! [`Val::Vw`](bevy::ui::Val) / [`Val::Vh`](bevy::ui::Val) sizes and a `min_height` so the
//! content cannot collapse to the empty-text minimum (the GTW-275 overflow root cause —
//! the auto-sized root mismeasured against zero-height `Text` and the rows rendered below
//! the border). Name / magazine seed with a non-empty placeholder so the first-frame
//! measure is correct. The content is HIDDEN via [`Display::None`] (removed from layout, so
//! a hidden weapon block takes no space — GTW-295), revealed + repainted from the selection
//! by [`update_weapon_panel`](super::update::update_weapon_panel) every battle frame,
//! mutating the existing widgets ([[ui-mutate-not-respawn]]).
//!
//! [`despawn_weapon_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the whole panel by its [`WeaponPanelRoot`] marker — battle-scoped, mirroring
//! the sibling status panel / action bar.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{Display, Node, UiRect, Val},
};
use gdtf_ui::{
    ButtonLabel, spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::scenes::running::game::battlescape::weapon_panel::components::{
    ReloadButton, WeaponContent, WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
};

/// The WIDTH of the landscape weapon-graphic placeholder, as a fraction of the WINDOW WIDTH
/// ([`Val::Vw`](bevy::ui::Val) — GTW-295 responsive ruling).
///
/// A `const`, layout plumbing fed straight to a [`Node`] (the `CELL_PX`-class carve-out,
/// not a domain value). Wider than its height so the graphic reads LANDSCAPE (the mockup's
/// horizontal weapon graphic), and `vw` so it scales with the window instead of a fixed px.
const GRAPHIC_W_VW: f32 = 9.0;

/// The HEIGHT of the weapon-graphic + throwable placeholder slots, as a fraction of the
/// WINDOW HEIGHT ([`Val::Vh`](bevy::ui::Val)).
///
/// A `const`, layout plumbing fed to a [`Node`]. Shorter than the graphic's width so the
/// graphic is landscape; the square throwable slots use this for BOTH edges.
const SLOT_H_VH: f32 = 6.0;

/// The minimum height of the weapon-content column, as a fraction of the WINDOW HEIGHT
/// ([`Val::Vh`](bevy::ui::Val)).
///
/// A `const`, layout plumbing fed to a [`Node`]. This is the GTW-275 overflow fix: it floors
/// the content height so the absolutely-positioned, auto-sized root cannot mismeasure against
/// near-zero-height text and push the rows below the panel border. Responsive (`vh`), NOT a
/// fixed px height.
const CONTENT_MIN_H_VH: f32 = 14.0;

/// The vertical gap between weapon-panel rows, in logical pixels (the stat-block
/// `ROW_GAP_PX` precedent). A `const`, layout plumbing fed to a [`Node`]. A small fixed gap
/// is the one justified px (a hairline-class spacing, not a scaling dimension).
const ROW_GAP_PX: f32 = 4.0;

/// Builds a WIDE LANDSCAPE PLACEHOLDER graphic box — a themed-panel-role [`Node`] with a
/// "no image available" caption, standing in for per-weapon art that does NOT exist (the
/// items atlas is deliberately not loaded, AC5). Sized [`GRAPHIC_W_VW`] × [`SLOT_H_VH`] so it
/// reads landscape (the mockup's horizontal weapon graphic), responsive.
///
/// Returns the box [`Entity`]; the caller parents it. It is a `Themed(Panel)` box so it reads
/// as an empty slot in the panel's look (re-painted by `apply_theme` like any themed node),
/// carrying a small centered caption.
fn spawn_graphic(commands: &mut Commands, theme: &GdtfTheme, caption: &str) -> Entity {
    spawn_slot(
        commands,
        theme,
        caption,
        Val::Vw(GRAPHIC_W_VW),
        Val::Vh(SLOT_H_VH),
    )
}

/// Builds a SQUARE throwable PLACEHOLDER slot — a themed-panel-role [`Node`] with a caption,
/// standing in for an unmodeled throwable (AC5). Both edges [`SLOT_H_VH`] so it stays square
/// and scales with the window.
fn spawn_throwable(commands: &mut Commands, theme: &GdtfTheme, caption: &str) -> Entity {
    spawn_slot(
        commands,
        theme,
        caption,
        Val::Vh(SLOT_H_VH),
        Val::Vh(SLOT_H_VH),
    )
}

/// Builds a themed placeholder slot box of `width` × `height` with a centered caption.
///
/// The shared body of [`spawn_graphic`] / [`spawn_throwable`] — a `Themed(Panel)` box (a
/// themed border / bg so it reads as an empty slot) holding one centered caption [`Text`].
fn spawn_slot(
    commands: &mut Commands,
    theme: &GdtfTheme,
    caption: &str,
    width: Val,
    height: Val,
) -> Entity {
    let slot = commands
        .spawn((
            Themed(ThemeRole::Panel),
            Node {
                width,
                height,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(*theme.panel.border_width_px)),
                ..default()
            },
            BackgroundColor(*theme.panel.color),
            bevy::ui::BorderColor::all(*theme.panel.border_color),
        ))
        .id();
    let label = commands
        .spawn((
            Themed(ThemeRole::Text),
            Text::new(caption),
            TextFont {
                font: theme.text.font.clone(),
                font_size: *theme.text.font_size_pt,
                ..default()
            },
            UiTextColor(*theme.text.text_color),
        ))
        .id();
    commands.entity(slot).add_children(&[label]);
    slot
}

/// Spawns a themed [`Text`] line tagged `marker`, started at `initial` — the name /
/// magazine lines (the stat-block `spawn_text` precedent). A NON-EMPTY `initial` seeds the
/// first-frame measure so the auto-sized root does not collapse against zero-height text
/// (the GTW-275 overflow contributing cause); the update mutates the text in place.
fn spawn_text(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    initial: &str,
) -> Entity {
    commands
        .spawn((
            marker,
            Themed(ThemeRole::Text),
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

/// Builds the themed weapon panel on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — in the running app the theme is present by the time a battle
/// starts; the status-panel precedent). With the theme present it spawns the
/// [`WeaponPanelRoot`] panel box anchored BOTTOM-LEFT (the free corner, AC5 / AC9) and fills
/// it with a single [`WeaponContent`] column laid out HORIZONTAL-FIRST to the mockup:
///
/// 1. a graphic ROW: a WIDE LANDSCAPE weapon-graphic placeholder beside the throwable
///    placeholder slots (AC5 / mockup horizontal-first cluster);
/// 2. an info column BELOW it: the [`WeaponNameText`] line, the [`WeaponMagazineText`]
///    `"cur/max"` line, and the LIVE [`ReloadButton`].
///
/// The content carries a responsive [`CONTENT_MIN_H_VH`] `min_height` so it cannot collapse
/// to the empty-text minimum (the GTW-275 overflow fix); name / magazine seed with a
/// non-empty placeholder so the first-frame measure is correct. The content is HIDDEN as a
/// unit via [`Display::None`] (removed from layout — GTW-295), revealed + repainted from the
/// selection by [`update_weapon_panel`](super::update::update_weapon_panel), MUTATING the
/// existing widgets ([[ui-mutate-not-respawn]]). Param-only (`bevy-traps.md` #7):
/// [`Commands`] + the theme read.
pub(in crate::scenes::running::game::battlescape) fn spawn_weapon_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed panel (the status-panel
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    // ROOT: the bottom-left anchored panel box (the FREE corner, AC5 / AC9). `spawn_panel`
    // paints the panel look; the layout fields survive `apply_theme` (it overrides only the
    // theme-owned border / radius / padding for the Panel role — the status-panel precedent).
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        WeaponPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));

    // 1. The graphic ROW (mockup horizontal-first): a WIDE landscape weapon graphic beside
    //    the throwable placeholder slots (AC5). Inert placeholders (no marker, no interaction)
    //    for art / a feature that is deliberately not modeled.
    let graphic = spawn_graphic(&mut commands, &theme, "no image");
    let throwable_a = spawn_throwable(&mut commands, &theme, "throw");
    let throwable_b = spawn_throwable(&mut commands, &theme, "throw");
    let graphic_row = commands
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(ROW_GAP_PX),
            align_items: AlignItems::Center,
            ..default()
        },))
        .id();
    commands
        .entity(graphic_row)
        .add_children(&[graphic, throwable_a, throwable_b]);

    // 2. The info column BELOW the graphic row: name, magazine cur/max, LIVE Reload.
    let name = spawn_text(&mut commands, &theme, WeaponNameText, "—");
    let magazine = spawn_text(&mut commands, &theme, WeaponMagazineText, "0/0");
    let reload = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Reload"),
        ReloadButton,
    );
    let info = commands
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(ROW_GAP_PX),
            ..default()
        },))
        .id();
    commands
        .entity(info)
        .add_children(&[name, magazine, reload]);

    // The content column holds the graphic row + the info column. A responsive `min_height`
    // floors its size so the absolutely-positioned auto-sized root cannot collapse against
    // near-zero-height text and overflow below the border (the GTW-275 fix). Hidden as a unit
    // via `Display::None` (removed from layout — GTW-295) until a weapon is selected (AC9);
    // it ALSO carries `Visibility::Hidden` so the existing visibility contract holds. The
    // update reveals it (display Flex + visibility Inherited).
    let content = commands
        .spawn((
            WeaponContent,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(ROW_GAP_PX),
                min_height: Val::Vh(CONTENT_MIN_H_VH),
                display: Display::None,
                ..default()
            },
            Visibility::Hidden,
        ))
        .id();
    commands.entity(content).add_children(&[graphic_row, info]);

    commands.entity(root).add_children(&[content]);
}

/// Despawns the weapon panel on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`WeaponPanelRoot`] entity (and so its children) so the panel
/// is gone the moment the battle leaves `BattleRunning` — battle-scoped lifecycle. Param-only
/// (`bevy-traps.md` #7): [`Commands`] + a `Query<Entity, With<WeaponPanelRoot>>`.
pub(in crate::scenes::running::game::battlescape) fn despawn_weapon_panel(
    mut commands: Commands,
    panels: Query<Entity, With<WeaponPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
