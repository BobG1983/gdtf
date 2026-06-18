//! Repaints the hover-inspect panel from the [`HoveredCell`](gdtf_battle_input::HoveredCell)
//! (GTW-274).
//!
//! [`update_hover_panel`] reads the hovered cell + the
//! [`OccupancyGrid`](gdtf_battle_sim::OccupancyGrid) (occupant / terrain) +
//! [`CoverLedger`](gdtf_battle_sim::CoverLedger) and drives the panel:
//!
//! - a hovered GANGER → show the shared stat block (the host) for it (with its NAME line
//!   color tinted by the ganger's `Faction` — enemy red-ish, player the normal theme),
//!   hide the object block;
//! - a hovered non-floor OBJECT (wall / cover) → show the object block (hardness + integrity),
//!   hide the stat block;
//! - bare floor / nothing → hide the whole panel.
//!
//! Every change is a mutate of the existing widgets ([[ui-mutate-not-respawn]]); the system
//! never writes the sim. It runs in `Update` gated `run_if(resource_exists::<BattleInProgress>)`
//! (`bevy-traps.md` #1), `.after(InputSystems::Gather)` so it observes the same update's hover
//! pick.

use bevy::{prelude::*, text::TextColor as UiTextColor};
use gdtf_battle_input::HoveredCell;
use gdtf_battle_sim::{
    CoverEntry, CoverLedger, Faction, OccupancyGrid, PlayerFaction, TerrainKind,
};
use gdtf_ui::{FillFraction, set_progress_bar, theme::GdtfTheme};

use crate::scenes::running::game::battlescape::{
    hover_panel::components::{
        HoverObjectBar, HoverObjectBlock, HoverObjectText, HoverPanelRoot, HoverStatBlockHost,
    },
    stat_block::{
        StatBlockData, StatBlockRefs, StatBlockWidgets, clear_stat_block, update_stat_block,
    },
};

/// The enemy-ganger name tint — a red-ish accent applied to the hover panel's name line
/// when the hovered ganger is NOT the player's faction (the contract's "enemy = red-ish"),
/// matching the mockup's red enemy panel.
///
/// A `const` [`Color`] fed straight to a [`TextColor`](bevy::text::TextColor) — the
/// `CELL_PX`-class framework-plumbing carve-out (`.claude/rules/no-bare-types.md` clause 4),
/// the same reasoning the stat-block bar/pip colors use. A name COLOR tint (not the faction
/// text line); the player faction keeps the normal theme text color.
const ENEMY_TINT: Color = Color::srgb(0.86, 0.26, 0.22);

/// The read-only marker→entity lookups the hover panel needs to find its own nodes.
///
/// A [`SystemParam`] bundle so the update system declares them as one param. All read-only
/// `Query<Entity, With<…>>` (the markers are disjoint), so no conflict with the
/// [`StatBlockWidgets`] write bundle. `pub(in …battlescape)` for `private_interfaces`.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::scenes::running::game::battlescape) struct HoverNodes<'w, 's> {
    /// The panel root (whole-panel visibility toggle).
    pub root:         Query<'w, 's, Entity, With<HoverPanelRoot>>,
    /// The shared stat-block host (the ganger sub-block visibility toggle).
    pub host:         Query<'w, 's, Entity, With<HoverStatBlockHost>>,
    /// The object block container (the object sub-block visibility toggle).
    pub object_block: Query<'w, 's, Entity, With<HoverObjectBlock>>,
    /// The object block's name/hardness `Text`.
    pub object_text:  Query<'w, 's, Entity, With<HoverObjectText>>,
    /// The object block's integrity `ProgressBar` track.
    pub object_bar:   Query<'w, 's, Entity, With<HoverObjectBar>>,
}

/// The sim-resource reads the hover panel resolves its hovered cell against.
///
/// A [`SystemParam`] bundle so the update system declares the cursor + grid + ledger reads
/// as ONE param (the [`too_many_arguments`](clippy::too_many_arguments) idiom). The grid
/// and ledger are [`Option`] (state-scoped — present only during a live battle,
/// `bevy-traps.md` #1). `pub(in …battlescape)` for `private_interfaces`.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::scenes::running::game::battlescape) struct HoverReads<'w> {
    /// The picked hovered cell (`gdtf_battle_input`).
    pub hovered: Res<'w, HoveredCell>,
    /// The occupancy / terrain grid (occupant lookup + terrain kind).
    pub grid:    Option<Res<'w, OccupancyGrid>>,
    /// The cover ledger (a hovered object's seeded structural stats).
    pub ledger:  Option<Res<'w, CoverLedger>>,
}

/// The faction-tint reads the hover panel needs to recolor the name line by the hovered
/// ganger's allegiance (AC2).
///
/// A [`SystemParam`] bundle so the update system declares the tint inputs as ONE param
/// (the [`too_many_arguments`](clippy::too_many_arguments) idiom — system analogue of the
/// ctor-struct fix). Both resources are [`Option`] (state-scoped — present only during a
/// live battle, `bevy-traps.md` #1); the [`UiTextColor`](bevy::text::TextColor) writer is
/// disjoint from every [`StatBlockWidgets`] query (a distinct component), so no conflict.
/// `pub(in …battlescape)` for `private_interfaces`.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::scenes::running::game::battlescape) struct FactionTint<'w, 's> {
    /// The player's faction — the hovered ganger is an ENEMY when its [`Faction`] differs.
    pub player: Option<Res<'w, PlayerFaction>>,
    /// The runtime theme — its body-text color is the player (normal) name color.
    pub theme:  Option<Res<'w, GdtfTheme>>,
    /// The name-line [`TextColor`](bevy::text::TextColor) writer (the tint target).
    pub colors: Query<'w, 's, &'static mut UiTextColor>,
}

/// Repaints the hover panel from the current [`HoveredCell`].
///
/// Resolves the hovered cell to a ganger (the grid's
/// [`occupant`](gdtf_battle_sim::OccupancyGrid::occupant) with [`StatBlockData`]), else a
/// non-floor object (the grid's [`terrain`](gdtf_battle_sim::OccupancyGrid::terrain) being a
/// wall / cover, with its seeded [`CoverEntry`] from the ledger), else bare floor; and toggles
/// the panel + its two sub-blocks accordingly. On a ganger hover it ALSO tints the name line
/// by the ganger's [`Faction`] (AC2 — enemy red-ish, player the normal theme color). Param-only
/// (`bevy-traps.md` #7).
pub(in crate::scenes::running::game::battlescape) fn update_hover_panel(
    reads: HoverReads,
    blocks: Query<&StatBlockRefs, With<HoverStatBlockHost>>,
    data: Query<StatBlockData>,
    mut widgets: StatBlockWidgets,
    nodes: HoverNodes,
    mut tint: FactionTint,
) {
    let Ok(&refs) = blocks.single() else {
        return;
    };

    // What is under the cursor: a ganger entity, a non-floor object's cover entry, or
    // bare floor / nothing. Each branch toggles the panel + sub-blocks (mutate-in-place).
    let cell = **reads.hovered;
    let occupant = cell.and_then(|c| reads.grid.as_deref().and_then(|g| g.occupant(&c)));

    if let Some(ganger) = occupant.and_then(|e| data.get(e).ok()) {
        // A hovered GANGER → show the panel + the ganger sub-block, hide the object block.
        toggle(&mut widgets.visibility, &nodes.root, Visibility::Inherited);
        toggle(&mut widgets.visibility, &nodes.host, Visibility::Inherited);
        toggle(
            &mut widgets.visibility,
            &nodes.object_block,
            Visibility::Hidden,
        );
        // AC2 faction tint: recolor the name line — enemy red-ish, player the normal theme.
        tint_name(refs.name, *ganger.faction, &mut tint);
        update_stat_block(refs, &ganger, &mut widgets);
        return;
    }

    if let Some(entry) =
        cell.and_then(|c| object_entry(c, reads.grid.as_deref(), reads.ledger.as_deref()))
    {
        // A hovered non-floor OBJECT → show the panel + the object block, hide the ganger block.
        toggle(&mut widgets.visibility, &nodes.root, Visibility::Inherited);
        toggle(&mut widgets.visibility, &nodes.host, Visibility::Hidden);
        toggle(
            &mut widgets.visibility,
            &nodes.object_block,
            Visibility::Inherited,
        );
        // Clear the ganger block so it carries no stale data while hidden.
        clear_stat_block(refs, &mut widgets);
        fill_object_block(&mut widgets, &nodes, entry);
        return;
    }

    // Bare floor / nothing hovered → hide the whole panel.
    toggle(&mut widgets.visibility, &nodes.root, Visibility::Hidden);
}

/// Tints the name line `Entity`'s [`TextColor`](bevy::text::TextColor) by `faction` (AC2):
/// the [`ENEMY_TINT`] red-ish when the hovered ganger is NOT the player's faction, else the
/// runtime theme's normal body-text color (the "normal theme" the contract asks for) —
/// mutate-in-place, only when the color differs ([[ui-mutate-not-respawn]]).
///
/// Reads the player faction + theme defensively as [`Option`] (state-scoped, present only in
/// a live battle, `bevy-traps.md` #1). With no [`PlayerFaction`] resource the tint falls back
/// to the normal theme color (no allegiance known → no enemy highlight); with no
/// [`GdtfTheme`] the player color falls back to the bevy default text color. The hover update
/// re-asserts this every frame, so it reclaims the color after any `apply_theme` repaint.
fn tint_name(name: Entity, faction: Faction, tint: &mut FactionTint) {
    let is_enemy = tint.player.as_deref().is_some_and(|p| **p != faction);
    let normal = tint
        .theme
        .as_deref()
        .map_or_else(|| *UiTextColor::default(), |t| *t.text.text_color);
    let want = if is_enemy { ENEMY_TINT } else { normal };
    if let Ok(mut color) = tint.colors.get_mut(name)
        && color.0 != want
    {
        color.0 = want;
    }
}

/// The cover/object entry for a hovered cell, if it is a non-floor object — `None` for bare
/// floor (so the panel hides).
///
/// A cell is an OBJECT when its [`TerrainKind`] is [`Wall`](TerrainKind::Wall) or
/// [`Cover`](TerrainKind::Cover). Its structural stats come from the
/// [`CoverLedger`](gdtf_battle_sim::CoverLedger) `peek` (seeded at setup for every authored
/// piece); a wall with no ledger entry falls back to a default full-integrity entry so the
/// object block still renders a name. `None` (the floor case) hides the panel.
fn object_entry(
    cell: gdtf_battle_sim::CellLevel,
    grid: Option<&OccupancyGrid>,
    ledger: Option<&CoverLedger>,
) -> Option<CoverEntry> {
    let grid = grid?;
    let terrain = grid.terrain(&cell);
    if !matches!(terrain, TerrainKind::Wall | TerrainKind::Cover) {
        return None;
    }
    // Prefer the seeded ledger entry; fall back to a minimal full-integrity entry for a
    // bare wall the ledger never registered (so the block still renders a name + a full bar).
    ledger.and_then(|l| l.peek(&cell).copied()).or_else(|| {
        Some(CoverEntry::seeded(
            gdtf_battle_sim::CoverHp::new(1),
            gdtf_battle_sim::HeightBand::High,
            gdtf_battle_sim::ArmorProtection::new(0),
            gdtf_battle_sim::ArmorHardness::new(0),
        ))
    })
}

/// Fills the object block's name/hardness `Text` + integrity `ProgressBar` from a
/// [`CoverEntry`] — mutate-in-place ([[ui-mutate-not-respawn]]).
fn fill_object_block(widgets: &mut StatBlockWidgets, nodes: &HoverNodes, entry: CoverEntry) {
    if let Some(text) = nodes.object_text.iter().next() {
        let label = format!("Cover · Hardness {}", *entry.armor_hardness);
        if let Ok(mut t) = widgets.texts.get_mut(text)
            && t.as_str() != label
        {
            label.clone_into(&mut t.0);
        }
    }
    if let Some(bar) = nodes.object_bar.iter().next() {
        let fraction =
            FillFraction::from_ratio(hp_as_f32(*entry.current_hp), hp_as_f32(*entry.max_hp));
        set_progress_bar(bar, fraction, &widgets.children, &mut widgets.fills);
    }
}

/// A cover-HP magnitude (`u32`) as an `f32` for the bar ratio — clamped through `u16` so the
/// conversion is lossless and lint-clean (no `cast_precision_loss`); cover HP is a small pool,
/// so the `u16` ceiling is never reached in practice and saturating it is harmless for a bar.
fn hp_as_f32(hp: u32) -> f32 {
    f32::from(u16::try_from(hp).unwrap_or(u16::MAX))
}

/// Sets the [`Visibility`] of the single entity yielded by `query` to `want`, only when it
/// differs (change-detection hygiene). A no-match (panel not spawned) is a graceful no-op.
fn toggle(
    visibility: &mut Query<&mut Visibility>,
    query: &Query<Entity, impl bevy::ecs::query::QueryFilter>,
    want: Visibility,
) {
    if let Some(entity) = query.iter().next()
        && let Ok(mut vis) = visibility.get_mut(entity)
        && *vis != want
    {
        *vis = want;
    }
}
