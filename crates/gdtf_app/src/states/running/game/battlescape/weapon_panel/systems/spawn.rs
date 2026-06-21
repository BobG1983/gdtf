//! Spawns + despawns the battlescape weapon cluster (GTW-275 / GTW-295 / GTW-298, bottom-left).
//!
//! [`spawn_weapon_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a themed
//! [`gdtf_ui`] panel group on the GTW-120 UI camera to the AUTHORITATIVE layout (GTW-298, user
//! 2026-06-18): the [`WeaponPanelRoot`] **Overall Weapon Panel** box anchored BOTTOM-LEFT and
//! sitting IN the bottom bar (its height = the bar height, drawn one z ABOVE the bar — GTW-275
//! layout overhaul item 6: no bleed out the top), laid out as a 2×2 grid (every size a RELATIVE
//! unit — `Percent` of parent / `Vw` / `Vh` / flex — no fixed px but the hairline border):
//!
//! - **LEFT column** (3/4 width): the [`CombinedWeaponPanel`] (top 3/4 height) over the Firemode
//!   Panel (bottom 1/4 height);
//! - **RIGHT column** (1/4 width): the [`WeaponItemPanel`] (top 3/4 height) over the [`AimPanel`]
//!   (bottom 1/4 height).
//!
//! The **Combined Weapon Panel** is ONE bordered box: a FULL-WIDTH [`WeaponImage`] placeholder
//! (top 1/2 height) over an info row of [the [`WeaponContent`] weapon-text column (name +
//! magazine, the FLEX SPONGE) | the LIVE [`ReloadButton`] (pinned right)] (bottom 1/2 height).
//! The **Item Panel** holds two stacked DISABLED [`WeaponItemButton`]s (1/2 height each, full
//! width). The **Firemode / Aim** panels host the controls RELOCATED from the action bar
//! (GTW-298): the firemode 3-toggle [`spawn_mode_panel`] column fills the Firemode cell, and the
//! [`AimToggleButton`] fills the [`AimPanel`].
//!
//! A SEPARATE **Stance Panel** ([`spawn_stance_panel`]) sits to the RIGHT of the Overall Weapon
//! Panel — its OWN bordered `Themed(Panel)` box (D-B, screenshot review 2026-06-18), sized to the
//! SAME HEIGHT as the Overall Weapon Panel ([`PANEL_H_VH`], NOT the full bottom-bar height) at a
//! fixed relative width ([`STANCE_W_VW`]), wrapping the three relocated stance toggles. It is
//! parented as a CHILD of the [`BottomBarRoot`] container — so it is laid out INSIDE the bottom
//! panel, contained by the bar's bounds + stacking context, NOT a free-floating overlay sitting ON
//! TOP of the bottom panel's edge. Its fixed-% width keeps it from changing the Overall panel's own
//! width (item 7 — the map area is stable). The relocated controls keep their action-bar markers,
//! so the existing press → intent router + active-mark syncs drive them unchanged.
//!
//! Sizing is RESPONSIVE (GTW-295): the root carries window-relative [`Val::Vw`](bevy::ui::Val)
//! width + [`Val::Vh`](bevy::ui::Val) height, the columns / cells split it by `Percent`, and the
//! weapon-text column carries a `min_height` so it cannot collapse to the empty-text minimum.
//! The weapon-text column is HIDDEN via [`Display::None`] when there is no selection / no weapon,
//! revealed + repainted from the selection by
//! [`update_weapon_panel`](super::update::update_weapon_panel) every battle frame, mutating the
//! existing widgets ([[ui-mutate-not-respawn]]).
//!
//! [`despawn_weapon_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the whole cluster by its [`WeaponPanelRoot`] + Stance-panel markers — battle-scoped,
//! mirroring the sibling status panel / action bar.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Display, GlobalZIndex, Node, Overflow, OverflowAxis, UiRect, Val},
};
use gdtf_ui::{
    ButtonLabel, DisabledButton, spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::running::game::battlescape::{
    action_bar::{spawn_aim_button, spawn_mode_panel, spawn_stance_panel},
    bottom_bar::{BOTTOM_BAR_H_VH, BOTTOM_BAR_PAD_Y_VH, BottomBarRoot},
    weapon_panel::components::{
        AimLabel, AimPanel, CombinedWeaponPanel, ReloadButton, WeaponContent, WeaponImage,
        WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
    },
};

/// The WIDTH of the Overall Weapon Panel, as a fraction of the WINDOW WIDTH
/// ([`Val::Vw`](bevy::ui::Val) — GTW-295 responsive ruling).
///
/// A `const`, layout plumbing fed straight to a [`Node`] (the `CELL_PX`-class carve-out, not a
/// domain value). The panel occupies the LEFT region of the bottom bar in the mockup; `vw` so
/// it scales with the window. A FIXED fraction (item 7): the renderable map area does not
/// pop/shift when the panel contents change.
const PANEL_W_VW: f32 = 22.0;

/// The WIDTH of the separate Stance Panel, as a fraction of the WINDOW WIDTH
/// ([`Val::Vw`](bevy::ui::Val)).
///
/// A `const`, layout plumbing fed to a [`Node`]. The Stance panel sits to the RIGHT of the
/// Overall Weapon Panel (a narrow stacked-toggle column in the mockup); a fixed `vw` fraction
/// so the map area is stable (item 7).
const STANCE_W_VW: f32 = 8.0;

/// The LEFT edge of the separate Stance Panel, as a fraction of the WINDOW WIDTH — placed
/// immediately to the RIGHT of the Overall Weapon Panel (a small gap beyond [`PANEL_W_VW`]).
///
/// A `const`, layout plumbing fed to a [`Node`]: `PANEL_W_VW` + a hairline gap so the Stance
/// panel does not overlap the Overall panel (item 8 — no panel overlaps another).
const STANCE_LEFT_VW: f32 = PANEL_W_VW + 0.5;

/// The weapon cluster's stacking order ([`GlobalZIndex`]) — one ABOVE the bottom bar's, so the
/// panels (which sit IN the bar, item 6) draw on TOP of the bar's opaque fill.
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`] (the framework carve-out, not
/// a domain value). Kept in lockstep with the bar's z (`bottom_bar` = 10) — the panels must be
/// strictly greater.
const PANEL_Z: i32 = 11;

/// The HEIGHT of the weapon cluster, as a fraction of the WINDOW HEIGHT
/// ([`Val::Vh`](bevy::ui::Val)).
///
/// Sized to the bar's CONTENT box (the bar height [`BOTTOM_BAR_H_VH`] MINUS its top + bottom
/// padding [`BOTTOM_BAR_PAD_Y_VH`] each) so the cluster insets off BOTH vertical edges of the bar
/// (D5 — 2026-06-18 screenshot review). The cluster is an ABSOLUTE overlay anchored to the window,
/// not an in-flow child of the bar, so the bar's `Node::padding` does NOT inset it — the bottom /
/// top breathing room has to live in the cluster's OWN geometry (anchored `bottom:
/// `[`BOTTOM_BAR_PAD_Y_VH`] with this reduced height), or the bottom row (Firemode toggles / Aim)
/// sits flush against the window's bottom edge (P1/P2). It still cannot bleed out the top of the
/// bar (GTW-275 layout overhaul item 6) — it now stops a padding short of BOTH edges. Gives the
/// grid cells a concrete parent to take their `Percent` shares of (a `Column` with `height: auto`
/// would not resolve child `Percent` heights). Responsive (`vh`), NOT a fixed px height.
const PANEL_H_VH: f32 = BOTTOM_BAR_H_VH - 2.0 * BOTTOM_BAR_PAD_Y_VH;

/// The COMBINED / ITEM cell height as a `Percent` of its column — the top band (the authoritative
/// height split). A `const` layout plumbing value, NOT a fixed px.
///
/// GTW-303 clip fix (2026-06-19): shrunk from 75 → 65 % (and [`BOTTOM_CELL_PCT`] grown to match)
/// so the bottom firemode / aim row gets a larger share of the column. The firemode segments are
/// now TWO lines (the mode name over its `"{n} TU"` cost sub-line — GTW-303); the larger bottom
/// cell gives the second line the vertical room it needs so it is not clipped. The Combined / Item
/// cells stay comfortably tall for the image + name/mag/Reload row at 65 %.
const TOP_CELL_PCT: f32 = 65.0;

/// The FIREMODE / AIM cell height as a `Percent` of its column — the bottom band (the
/// authoritative height split). A `const` layout plumbing value, NOT a fixed px.
///
/// GTW-303 clip fix (2026-06-19): grown from 25 → 35 % (mirroring [`TOP_CELL_PCT`]'s shrink) so
/// the firemode control's now-TWO-line segments (mode name over the `"{n} TU"` cost sub-line —
/// GTW-303) have the vertical room to render BOTH lines fully, instead of the cost line clipping
/// at the cell's bottom edge.
const BOTTOM_CELL_PCT: f32 = 35.0;

/// The LEFT column width as a `Percent` of the Overall panel — the 3/4 share (Combined +
/// Firemode). A `const` layout plumbing value, NOT a fixed px.
const LEFT_COL_PCT: f32 = 75.0;

/// The RIGHT column width as a `Percent` of the Overall panel — the 1/4 share (Item + Aim). A
/// `const` layout plumbing value, NOT a fixed px.
const RIGHT_COL_PCT: f32 = 25.0;

/// The weapon-text column / Reload's height as a `Percent` of the Combined panel — the bottom
/// 1/2 (the [`WeaponImage`] takes the top 1/2). A `const` layout plumbing value, NOT a fixed px.
const INFO_ROW_H_PCT: f32 = 50.0;

/// The weapon-text column's width as a `Percent` of the info row — the 3/4 share beside the
/// 1/4-width Reload button. A `const` layout plumbing value, NOT a fixed px.
const TEXT_COL_PCT: f32 = 75.0;

/// The minimum height of the weapon-text column, as a fraction of the WINDOW HEIGHT
/// ([`Val::Vh`](bevy::ui::Val)).
///
/// A `const`, layout plumbing fed to a [`Node`]. The GTW-275 overflow floor: it floors the
/// weapon-text height so the absolutely-positioned, auto-sized root cannot mismeasure against
/// near-zero-height text and push the rows below the panel border. Responsive (`vh`), NOT a
/// fixed px.
const CONTENT_MIN_H_VH: f32 = 3.0;

/// The VERTICAL gap between weapon-cluster sub-nodes (row gaps), as a fraction of the window
/// HEIGHT ([`Val::Vh`](bevy::ui::Val) — GTW-296 responsive ruling). A `const`, layout plumbing
/// fed to a [`Node`]. Calibrated to a 4px gap at the 1280x720 reference window (4 / 720 =
/// 0.55556) so the cluster is visually identical at the default size.
const GAP_VH: f32 = 0.55556;

/// The HORIZONTAL gap between weapon-cluster sub-nodes (column gaps), as a fraction of the window
/// WIDTH ([`Val::Vw`](bevy::ui::Val) — GTW-296 responsive ruling). A `const`, layout plumbing fed
/// to a [`Node`]. Calibrated to a 4px gap at the 1280x720 reference window (4 / 1280 = 0.3125) so
/// the cluster is visually identical at the default size.
const GAP_VW: f32 = 0.3125;

/// Builds an EMPTY FRAMED placeholder box tagged `marker`, sized `width` × `height`.
///
/// A `Themed(ThemeRole::Panel)` box (a themed border / bg / radius re-painted by `apply_theme`
/// like any themed node, so it reads as a framed box). Used for the Combined panel, the Item
/// panel, the Aim panel, and the image placeholder shell. Returns its [`Entity`].
fn spawn_frame(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    width: Val,
    height: Val,
) -> Entity {
    // GTW-322 — `Themed` rides the `bsn!` macro inline; the runtime-valued `Node`,
    // `BackgroundColor`, and `BorderColor` are composed with `template_value` (the
    // builder `BorderColor::all` is a method call, which the inline `CompA(expr)` grammar
    // rejects, so it is precomputed into a value); the generic `marker` is `.insert`ed.
    let node = Node {
        width,
        height,
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    };
    let background = BackgroundColor(*theme.panel.color);
    let border = bevy::ui::BorderColor::all(*theme.panel.border_color);
    commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Panel) },
            template_value(node),
            template_value(background),
            template_value(border),
        ))
        .insert(marker)
        .id()
}

/// Builds the FULL-WIDTH [`WeaponImage`] placeholder — a framed box with a small centered
/// "no image" caption, spanning the Combined panel's full width at `height` (a [`Val::Percent`]
/// of the panel's top half). Standing in for per-weapon art that does NOT exist.
fn spawn_image(commands: &mut Commands, theme: &GdtfTheme, height: Val) -> Entity {
    // GTW-322 — `WeaponImage` + `Themed` ride the `bsn!` macro inline; the runtime-valued
    // `Node`, `BackgroundColor`, and `BorderColor` are composed with `template_value`.
    let slot_node = Node {
        width: Val::Percent(100.0),
        height,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    };
    let background = BackgroundColor(*theme.panel.color);
    let border = bevy::ui::BorderColor::all(*theme.panel.border_color);
    let slot = commands
        .spawn_scene((
            bsn! {
                WeaponImage
                Themed::new(ThemeRole::Panel)
            },
            template_value(slot_node),
            template_value(background),
            template_value(border),
        ))
        .id();
    // The "no image" caption: `Themed` + `Text` + `UiTextColor` inline, `TextFont` (not
    // `Unpin`) on the `template(|_| ..)` closure.
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    let label = commands
        .spawn_scene(bsn! {
            Themed::new(ThemeRole::Text)
            Text::new("no image")
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .id();
    commands.entity(slot).add_children(&[label]);
    slot
}

/// Spawns the Aim Panel's **"Aim" caption** [`Text`] ([`AimLabel`]) — the static label that sits
/// to the LEFT of the relocated Aim [`Switch`](gdtf_ui::Switch) so the control reads
/// "Aim [switch]" (the mockup; the GTW-277 widget migration had dropped this caption). Returns
/// its [`Entity`].
///
/// A `Themed(ThemeRole::Text)` line (the "no image" caption precedent), so `apply_theme` paints
/// its font + color from the theme like any themed text. Its [`AimLabel`] marker is added to
/// [`fit_weapon_panel`](super::fit::fit_weapon_panel)'s label-owner set, which holds the caption
/// at the 14 pt control-label size `.after(UiSystems::ApplyTheme)` — matching the firemode /
/// stance segment captions (`nowrap_control_labels`) so the whole control cluster's labels read
/// at one size. A non-empty seed feeds the first-frame measure; the caption is static (never
/// mutated).
fn spawn_aim_label(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    // GTW-322 — `AimLabel` + `Themed` + `Text` + `UiTextColor` ride the `bsn!` macro
    // inline; `TextFont` (not `Unpin`) rides the `template(|_| ..)` closure.
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            AimLabel
            Themed::new(ThemeRole::Text)
            Text::new("Aim")
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .id()
}

/// Spawns a themed [`Text`] line tagged `marker`, started at `initial` (the stat-block
/// `spawn_text` precedent). A NON-EMPTY `initial` seeds the first-frame measure; the update
/// mutates the text in place.
fn spawn_text(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    initial: &str,
) -> Entity {
    // GTW-322 — `Themed` + `Text::new` + `UiTextColor` ride the `bsn!` macro inline;
    // `TextFont` (not `Unpin`) rides the `template(|_| ..)` closure; the generic `marker`
    // is `.insert`ed. The seed string is owned (`'static`) for the deferred apply.
    let text_color = *theme.text.text_color;
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

/// Builds the Combined panel's INFO ROW (bottom 1/2) — the [`WeaponContent`] weapon-text column
/// (name + magazine, 3/4 width) beside the LIVE [`ReloadButton`] (1/4 width).
///
/// The text column `flex_grow`s / CLIPS (`min_width: 0` + [`Overflow`] hidden) rather than
/// pushing Reload; the Reload cell is `flex_shrink: 0` (never collapses) and pinned to the
/// row's RIGHT END, so Reload can never float past the Combined panel's right edge into the Item
/// panel (contract: Reload INSIDE the Combined panel). Returns the info-row entity.
fn spawn_info_row(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let name = spawn_text(commands, theme, WeaponNameText, "—");
    let magazine = spawn_text(commands, theme, WeaponMagazineText, "0/0");
    // GTW-322 — `WeaponContent` rides the `bsn!` macro inline; its runtime-valued `Node`
    // (the flex-sponge text column) + `Visibility::Hidden` are composed with `template_value`.
    let content_node = Node {
        width: Val::Percent(TEXT_COL_PCT),
        height: Val::Percent(100.0),
        // The GTW-275 overflow floor: a RESPONSIVE (`Val::Vh`) min height so the
        // auto-sized panel cannot mismeasure the weapon-text against near-zero text.
        min_height: Val::Vh(CONTENT_MIN_H_VH),
        flex_grow: 1.0,
        flex_shrink: 1.0,
        min_width: Val::ZERO,
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        row_gap: Val::Vh(GAP_VH),
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        // Hidden as a unit when there is no selection / no weapon (AC9 empty state):
        // Display::None (removed from layout) + Visibility::Hidden. The update reveals it.
        display: Display::None,
        ..default()
    };
    let content = commands
        .spawn_scene((
            bsn! { WeaponContent },
            template_value(content_node),
            template_value(Visibility::Hidden),
        ))
        .id();
    commands.entity(content).add_children(&[name, magazine]);

    let reload = spawn_button(commands, theme, ButtonLabel::new("Reload"), ReloadButton);
    // GTW-322 — plain layout `Node`s (no markers); composed via `template_value`.
    let reload_cell_node = Node {
        width: Val::Percent(100.0 - TEXT_COL_PCT),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::FlexEnd,
        align_items: AlignItems::Center,
        ..default()
    };
    let reload_cell = commands.spawn_scene(template_value(reload_cell_node)).id();
    commands.entity(reload_cell).add_children(&[reload]);

    let info_row_node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(INFO_ROW_H_PCT),
        flex_direction: FlexDirection::Row,
        column_gap: Val::Vw(GAP_VW),
        align_items: AlignItems::Center,
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        ..default()
    };
    let info_row = commands.spawn_scene(template_value(info_row_node)).id();
    commands
        .entity(info_row)
        .add_children(&[content, reload_cell]);
    info_row
}

/// Builds the **Combined Weapon Panel** (top-left grid cell) — ONE bordered box of the
/// FULL-WIDTH [`WeaponImage`] (top 1/2) over the info row (bottom 1/2: weapon-text column |
/// Reload). Sized `width` × `height` (responsive). Returns the panel [`Entity`].
fn spawn_combined_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
    width: Val,
    height: Val,
) -> Entity {
    let image = spawn_image(commands, theme, Val::Percent(INFO_ROW_H_PCT));
    let info_row = spawn_info_row(commands, theme);
    // GTW-322 — `CombinedWeaponPanel` + `Themed` ride the `bsn!` macro inline; the
    // runtime-valued `Node`, `BackgroundColor`, and `BorderColor` are composed with
    // `template_value`.
    let panel_node = Node {
        width,
        height,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    };
    let background = BackgroundColor(*theme.panel.color);
    let border = bevy::ui::BorderColor::all(*theme.panel.border_color);
    let panel = commands
        .spawn_scene((
            bsn! {
                CombinedWeaponPanel
                Themed::new(ThemeRole::Panel)
            },
            template_value(panel_node),
            template_value(background),
            template_value(border),
        ))
        .id();
    commands.entity(panel).add_children(&[image, info_row]);
    panel
}

/// Builds the **Item Panel** (top-right grid cell) — a framed box holding two stacked DISABLED
/// [`WeaponItemButton`]s (1/2 height each, full width). Sized `width` × `height`. Returns the
/// panel [`Entity`].
fn spawn_item_panel(commands: &mut Commands, theme: &GdtfTheme, width: Val, height: Val) -> Entity {
    let panel = spawn_frame(commands, theme, WeaponItemPanel, width, height);
    let item_a = spawn_item_button(commands, theme);
    let item_b = spawn_item_button(commands, theme);
    commands.entity(panel).insert(Node {
        width,
        height,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    });
    commands.entity(panel).add_children(&[item_a, item_b]);
    panel
}

/// Spawns one DISABLED [`WeaponItemButton`] (full width, 1/2 height of the Item panel) — a
/// visible-but-non-interactable placeholder (items are not modeled yet). It carries
/// [`DisabledButton`](gdtf_ui::DisabledButton) so `gdtf_ui` paints it in the disabled color and
/// the `Without<DisabledButton>` interaction filters exclude it. Returns the button.
fn spawn_item_button(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let button = spawn_button(
        commands,
        theme,
        ButtonLabel::new("Item"),
        (WeaponItemButton, DisabledButton),
    );
    commands.entity(button).insert(Node {
        width: Val::Percent(100.0),
        flex_grow: 1.0,
        flex_basis: Val::Percent(0.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    });
    button
}

/// Builds the LEFT column (3/4 width) — the [`CombinedWeaponPanel`] (top 3/4 height) over the
/// relocated Firemode panel ([`spawn_mode_panel`], bottom 1/4 height). Returns the column
/// [`Entity`].
fn spawn_left_column(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let combined = spawn_combined_panel(
        commands,
        theme,
        Val::Percent(100.0),
        Val::Percent(TOP_CELL_PCT),
    );
    // The Firemode panel = the relocated action-bar Mode 3-toggle column (GTW-298). It fills the
    // bottom 1/4 cell; `rebuild_mode_buttons` shows the offered modes + the panel root.
    //
    // `spawn_mode_panel` already lays the panel out as a full-size `Row` that CLIPS its toggle row
    // (`overflow: Hidden`, so a wide label never overflows the cell into a sibling — item 8). We
    // only need to constrain its HEIGHT to the bottom 1/4 cell here, so we MUTATE just that field
    // on the existing `Node` rather than re-inserting a fresh one — a wholesale `insert(Node {
    // ..default() })` would silently DROP the panel's `overflow: Hidden` clip (and its `Row`
    // direction), letting the toggles overflow + overlap.
    let firemode = spawn_mode_panel(commands, theme);
    commands
        .entity(firemode)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.height = Val::Percent(BOTTOM_CELL_PCT);
        });
    // GTW-322 — a plain layout `Node` column (no markers); composed via `template_value`.
    let column_node = Node {
        width: Val::Percent(LEFT_COL_PCT),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        ..default()
    };
    let column = commands.spawn_scene(template_value(column_node)).id();
    commands.entity(column).add_children(&[combined, firemode]);
    column
}

/// Builds the RIGHT column (1/4 width) — the [`WeaponItemPanel`] (top 3/4 height) over the
/// [`AimPanel`] (bottom 1/4 height) wrapping the relocated Aim toggle. Returns the column
/// [`Entity`].
fn spawn_right_column(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let items = spawn_item_panel(
        commands,
        theme,
        Val::Percent(100.0),
        Val::Percent(TOP_CELL_PCT),
    );
    // The Aim panel = a framed cell wrapping the relocated action-bar Aim toggle (GTW-298),
    // laid out as a ROW: an "Aim" caption ([`AimLabel`]) on the LEFT, the toggle `Switch` on the
    // RIGHT — matching the mockup's "AIM [switch]" reading (the GTW-277 widget migration had
    // dropped the caption, leaving the bare switch). A ROW (not a column) was chosen because the
    // cell is the SHORT bottom 1/4-height of the right column: a horizontal "Aim [switch]" reads
    // cleaner in a short-and-wide box than stacking the caption over the already-horizontal
    // switch would in the tight vertical room. The cell centres the row
    // (`justify_content`/`align_items: Center`) with a small inter-child gap so the caption + the
    // self-contained switch (sized by its own track geometry, NOT a fill-the-box button) sit
    // together rather than the switch stretching to fill. The switch keeps its `AimToggleButton`
    // marker so the press → intent + sim-sync systems drive it parent-agnostically.
    let aim_panel = spawn_frame(
        commands,
        theme,
        AimPanel,
        Val::Percent(100.0),
        Val::Percent(BOTTOM_CELL_PCT),
    );
    commands
        .entity(aim_panel)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.flex_direction = FlexDirection::Row;
            n.justify_content = JustifyContent::Center;
            n.align_items = AlignItems::Center;
            n.column_gap = Val::Vw(GAP_VW);
        });
    let aim_label = spawn_aim_label(commands, theme);
    let aim_button = spawn_aim_button(commands, theme);
    commands
        .entity(aim_panel)
        .add_children(&[aim_label, aim_button]);
    // GTW-322 — a plain layout `Node` column (no markers); composed via `template_value`.
    let column_node = Node {
        width: Val::Percent(RIGHT_COL_PCT),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        ..default()
    };
    let column = commands.spawn_scene(template_value(column_node)).id();
    commands.entity(column).add_children(&[items, aim_panel]);
    column
}

/// Builds the themed weapon cluster on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — the status-panel precedent). With the theme present it spawns the
/// [`WeaponPanelRoot`] Overall Weapon Panel anchored BOTTOM-LEFT (sized responsively
/// [`PANEL_W_VW`] × [`PANEL_H_VH`]) as a 2×2 grid — [`spawn_left_column`] (Combined + Firemode)
/// beside [`spawn_right_column`] (Item + Aim) — and a SEPARATE Stance Panel
/// ([`spawn_stance_panel`], its OWN bordered `Themed(Panel)`) to its RIGHT (fixed-% width, height =
/// [`PANEL_H_VH`] — the SAME height as the Overall Weapon Panel, NOT the full bottom-bar height — D-B),
/// parented as a CHILD of the [`BottomBarRoot`] container so it is laid out INSIDE the
/// bottom panel rather than floating over it — and so all three stance toggles stay contained within
/// the bar's border/padding. The relocated controls (firemode / aim / stance) keep their action-bar markers, so the
/// existing press → intent + active-mark systems drive them. It is ordered
/// `.after(spawn_bottom_bar)` (the bottom-bar root must exist before the stance panel is parented
/// under it); if the bar root is somehow absent it falls back to parenting the stance under the
/// weapon root (so the toggles still spawn). Param-only (`bevy-traps.md` #7): [`Commands`], the
/// theme read, and a read-only `Query<Entity, With<BottomBarRoot>>`.
pub(in crate::states::running::game::battlescape) fn spawn_weapon_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    bottom_bar: Query<Entity, With<BottomBarRoot>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed panel (the status-panel
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    // ROOT: the bottom-left anchored Overall Weapon Panel box (a 2×2 grid), sized responsively
    // so the grid cells resolve their `Percent` shares. `spawn_panel` paints the panel look; the
    // layout fields survive `apply_theme` (it overrides only the theme-owned border / radius /
    // padding for the Panel role — the status-panel precedent).
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        WeaponPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            // Anchor the cluster a bottom-padding's worth ABOVE the WINDOW bottom (A FAIL,
            // 2026-06-18 screenshot review): this root is an absolute overlay positioned
            // relative to the WINDOW (it is NOT an in-flow child of the bar, so the bar's
            // `Node::padding` does NOT inset it) — at `bottom: 0` the Firemode toggles / Aim row
            // sat FLUSH against the window's bottom edge. Lift it by the bar's own bottom-padding
            // ([`BOTTOM_BAR_PAD_Y_VH`], a window-relative `Vh`) so the bottom row insets off the
            // window's bottom edge, matching the bar's content inset (the mockup's clear gap below
            // the bottom-panel content).
            bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
            left: Val::ZERO,
            width: Val::Vw(PANEL_W_VW),
            // Height = the bar's CONTENT-box height ([`PANEL_H_VH`] = bar height MINUS its top +
            // bottom padding); anchored a bottom-padding above the window bottom, its TOP lands a
            // top-padding below the bar's top edge — so it insets off BOTH vertical edges and never
            // bleeds out the top (GTW-275 layout overhaul item 6).
            height: Val::Vh(PANEL_H_VH),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Vw(GAP_VW),
            ..default()
        },
        // Draw ON TOP of the bottom bar's opaque fill (one z above the bar — item 6).
        GlobalZIndex(PANEL_Z),
    ));

    let left = spawn_left_column(&mut commands, &theme);
    let right = spawn_right_column(&mut commands, &theme);

    commands.entity(root).add_children(&[left, right]);

    // The SEPARATE Stance Panel (GTW-298; D-B per the 2026-06-18 screenshot review): its OWN
    // bordered `Themed(Panel)` box, sitting to the RIGHT of the Overall Weapon Panel at a fixed-%
    // width — so it does not change the Overall panel's own grid width (item 7) and does not overlap
    // it (item 8). It is parented as a CHILD of the [`BottomBarRoot`] container, so it is laid out
    // INSIDE the bottom panel (contained by the bar's bounds + stacking context) rather than
    // floating ON TOP of the bottom panel's right edge as a sibling overlay of the weapon root. It
    // is an absolute child of the bar whose `left` is relative to the bar's left edge (= the window
    // left, since the bar is full-width at `left: 0`), so the same `STANCE_LEFT_VW` geometry still
    // places it just past the weapon cluster. D-B: its height is `Vh(PANEL_H_VH)` — the SAME height
    // as the Overall Weapon Panel (NOT the full bottom-bar height) — anchored a bottom-padding ABOVE
    // the window bottom (`bottom: Vh(BOTTOM_BAR_PAD_Y_VH)`, matching the Overall Weapon Panel root —
    // B FAIL, 2026-06-18 screenshot review: at `bottom: 0` the absolute inset did NOT pick up the
    // bar's bottom padding, so the framed stance box ran flush to the window's bottom edge and read
    // as loose buttons on the bar fill). Since `PANEL_H_VH` is the bar CONTENT-box height
    // (`BOTTOM_BAR_H_VH - 2 * BOTTOM_BAR_PAD_Y_VH`), the column (and so the third 'Prone' toggle)
    // stays ENTIRELY inside the bar's border + padding instead of overrunning the bottom edge — and
    // the box's own bordered frame now sits clear of the window edge, reading as a distinct
    // sub-panel. It wraps the three relocated stance toggles; the `sync_stance_buttons_active` +
    // `action_bar_button_intents` systems drive them parent-agnostically. The bottom bar's own
    // `despawn_bottom_bar` tears it down with the bar (and if the fallback parents it under the
    // weapon root, `despawn_weapon_panel` does).
    let stance = spawn_stance_panel(&mut commands, &theme);
    commands.entity(stance).insert(Node {
        position_type: PositionType::Absolute,
        // D-B (2026-06-18 screenshot review): the Stance Panel is its OWN bordered sub-panel sized
        // to the SAME HEIGHT as the Overall Weapon Panel (NOT the full bottom-bar height) and a
        // sensible fixed relative width ([`STANCE_W_VW`]). It is anchored a bottom-padding ABOVE the
        // window bottom (`bottom: Vh(BOTTOM_BAR_PAD_Y_VH)` — the EXACT same `bottom` the Overall
        // Weapon Panel root uses) with `height: Vh(PANEL_H_VH)` — the same height field — so the two
        // bordered panels are flush-bottomed with each other AND both inset off the window's bottom
        // edge (B FAIL fix: `bottom: 0` left the framed box flush against the window edge). Since
        // `PANEL_H_VH` is the bar CONTENT-box height (`BOTTOM_BAR_H_VH - 2 * BOTTOM_BAR_PAD_Y_VH`),
        // the column (and so the third 'Prone' toggle) stays inside the bar's border + padding.
        bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
        left: Val::Vw(STANCE_LEFT_VW),
        width: Val::Vw(STANCE_W_VW),
        // Height = the Overall Weapon Panel's height EXACTLY (`Vh(PANEL_H_VH)`) — the bar
        // content-box height, so the column cannot overrun the bar's bottom border/padding.
        height: Val::Vh(PANEL_H_VH),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        ..default()
    });
    // Parent the stance panel INSIDE the bottom bar (D4). `spawn_weapon_panel` is ordered
    // `.after(spawn_bottom_bar)`, so the bar root exists; if it is somehow absent (defensive)
    // fall back to the weapon root so the stance toggles still spawn.
    let stance_parent = bottom_bar.iter().next().unwrap_or(root);
    commands.entity(stance_parent).add_children(&[stance]);
}

/// Despawns the weapon cluster on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`WeaponPanelRoot`] Overall Weapon Panel (and so its grid children)
/// so the cluster is gone the moment the battle leaves `BattleRunning` — battle-scoped lifecycle.
/// The Stance Panel is now a child of the [`BottomBarRoot`] (D4 reparent), so `despawn_bottom_bar`
/// tears it down with the bar; this still covers the defensive fallback where the bar was absent
/// and the stance was parented under the weapon root. Param-only (`bevy-traps.md` #7):
/// [`Commands`] + a `Query<Entity, With<WeaponPanelRoot>>`.
pub(in crate::states::running::game::battlescape) fn despawn_weapon_panel(
    mut commands: Commands,
    panels: Query<Entity, With<WeaponPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
