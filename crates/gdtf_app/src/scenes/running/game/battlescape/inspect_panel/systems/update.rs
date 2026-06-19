//! Repaints the inspect panel from the EFFECTIVE
//! [`InspectTarget`](gdtf_battle_input::InspectTarget) — pinned-else-hovered (GTW-274 / GTW-300).
//!
//! [`update_inspect_panel`] reads the effective inspect cell + the
//! [`OccupancyGrid`](gdtf_battle_sim::OccupancyGrid) (occupant / terrain) +
//! [`CoverLedger`](gdtf_battle_sim::CoverLedger) and drives the panel:
//!
//! - a hovered GANGER → show the shared stat block (the host) for it (with its NAME line
//!   color tinted by the ganger's `Faction` — enemy red-ish, player the normal theme),
//!   hide the object block;
//! - a hovered non-floor OBJECT (wall / cover) → show the OBJECT stat block (title + a labeled
//!   Integrity bar + labeled Hardness / Protection / Height-band lines, GTW-295), hide the
//!   ganger stat block;
//! - bare floor / nothing → hide the whole panel.
//!
//! The panel ROOT is hidden by `Visibility`; the two SUB-BLOCKS by `Node.display` (`None`
//! removes a hidden block from layout, so the panel sizes to the visible block only — GTW-295
//! fixes the cover-hover balloon). Every change is a mutate of the existing widgets
//! ([[ui-mutate-not-respawn]]); the system never writes the sim. It runs in `Update` gated
//! `run_if(resource_exists::<BattleInProgress>)` (`bevy-traps.md` #1), `.after(InputSystems::Gather)`
//! so it observes the same update's hover pick.

use bevy::{prelude::*, text::TextColor as UiTextColor, ui::Display};
use gdtf_battle_input::{InspectMode, InspectTarget};
use gdtf_battle_sim::{
    CoverEntry, CoverLedger, Faction, HeightBand, OccupancyGrid, PlayerFaction, TerrainKind,
};
use gdtf_ui::{FillFraction, ProgressBarFill, set_progress_bar, theme::GdtfTheme};

use crate::scenes::running::game::battlescape::{
    inspect_panel::components::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
    },
    stat_block::{
        StatBlockData, StatBlockRefs, StatBlockWidgets, clear_stat_block, update_stat_block,
    },
};

/// The enemy-ganger name tint — a red-ish accent applied to the inspect panel's name line
/// when the hovered ganger is NOT the player's faction (the contract's "enemy = red-ish"),
/// matching the mockup's red enemy panel.
///
/// A `const` [`Color`] fed straight to a [`TextColor`](bevy::text::TextColor) — the
/// `CELL_PX`-class framework-plumbing carve-out (`.claude/rules/no-bare-types.md` clause 4),
/// the same reasoning the stat-block bar/pip colors use. A name COLOR tint (not the faction
/// text line); the player faction keeps the normal theme text color.
const ENEMY_TINT: Color = Color::srgb(0.86, 0.26, 0.22);

/// The read-only marker→entity lookups the inspect panel needs to find its own nodes.
///
/// A [`SystemParam`] bundle so the update system declares them as one param. All read-only
/// `Query<Entity, With<…>>` (the markers are disjoint), so no conflict with the
/// [`StatBlockWidgets`] write bundle. `pub(in …battlescape)` for `private_interfaces`.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::scenes::running::game::battlescape) struct InspectNodes<'w, 's> {
    /// The panel root (whole-panel visibility toggle).
    pub root:         Query<'w, 's, Entity, With<InspectPanelRoot>>,
    /// The shared stat-block host (the ganger sub-block show/hide target).
    pub host:         Query<'w, 's, Entity, With<InspectStatBlockHost>>,
    /// The object block container (the object sub-block show/hide target).
    pub object_block: Query<'w, 's, Entity, With<InspectObjectBlock>>,
    /// The object block's **title** `Text` (object kind heading).
    pub object_title: Query<'w, 's, Entity, With<InspectObjectText>>,
    /// The object block's **Hardness** `Text` line.
    pub hardness:     Query<'w, 's, Entity, With<InspectObjectHardness>>,
    /// The object block's **Protection** `Text` line.
    pub protection:   Query<'w, 's, Entity, With<InspectObjectProtection>>,
    /// The object block's **Height band** `Text` line.
    pub height:       Query<'w, 's, Entity, With<InspectObjectHeight>>,
    /// The object block's integrity `ProgressBar` track.
    pub object_bar:   Query<'w, 's, Entity, With<InspectObjectBar>>,
    /// The sub-blocks' layout `Node` writer — drives `Display::Flex` / `Display::None` so a
    /// hidden sub-block is REMOVED from layout (GTW-295). Filtered `Without<ProgressBarFill>`
    /// so it stays disjoint from the [`StatBlockWidgets`] `fills` `&mut Node` writer.
    pub display:      Query<'w, 's, &'static mut Node, Without<ProgressBarFill>>,
}

/// The sim-resource reads the inspect panel resolves its EFFECTIVE target against (GTW-300).
///
/// A [`SystemParam`] bundle so the update system declares the inspect target + grid + ledger
/// reads as ONE param (the [`too_many_arguments`](clippy::too_many_arguments) idiom). The grid
/// and ledger are [`Option`] (state-scoped — present only during a live battle,
/// `bevy-traps.md` #1). `pub(in …battlescape)` for `private_interfaces`.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::scenes::running::game::battlescape) struct InspectReads<'w> {
    /// The inspect target (`gdtf_battle_input`) — the panel reads its EFFECTIVE mode
    /// (pinned-else-hovered, GTW-300), NOT the raw cursor cell.
    pub target: Res<'w, InspectTarget>,
    /// The occupancy / terrain grid (occupant lookup + terrain kind).
    pub grid:   Option<Res<'w, OccupancyGrid>>,
    /// The cover ledger (a hovered object's seeded structural stats).
    pub ledger: Option<Res<'w, CoverLedger>>,
}

/// The faction-tint reads the inspect panel needs to recolor the name line by the hovered
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

/// Repaints the inspect panel from the current EFFECTIVE [`InspectTarget`] (pinned-else-hovered,
/// GTW-300): a pinned cell freezes the panel on its occupant / cover; with no pin it follows the
/// live hovered cell.
///
/// Resolves the effective cell to a ganger (the grid's
/// [`occupant`](gdtf_battle_sim::OccupancyGrid::occupant) with [`StatBlockData`]), else a
/// non-floor object (the grid's [`terrain`](gdtf_battle_sim::OccupancyGrid::terrain) being a
/// wall / cover, with its seeded [`CoverEntry`] from the ledger), else bare floor; and toggles
/// the panel + its two sub-blocks accordingly. On a ganger hover it ALSO tints the name line
/// by the ganger's [`Faction`] (AC2 — enemy red-ish, player the normal theme color). Param-only
/// (`bevy-traps.md` #7).
pub(in crate::scenes::running::game::battlescape) fn update_inspect_panel(
    reads: InspectReads,
    blocks: Query<&StatBlockRefs, With<InspectStatBlockHost>>,
    data: Query<StatBlockData>,
    mut widgets: StatBlockWidgets,
    mut nodes: InspectNodes,
    mut tint: FactionTint,
) {
    let Ok(&refs) = blocks.single() else {
        return;
    };

    // What the panel describes: the EFFECTIVE inspect target (pinned-else-hovered, GTW-300),
    // resolved to a sim cell. With a pin, `effective()` is `Pinned(cell)` and the panel freezes
    // on that cell's occupant / cover; with no pin it is `Hovered(cell)`, following the cursor.
    // Either way the cell flows through the SAME occupant / terrain resolution below. Each
    // branch below toggles the panel + sub-blocks (mutate-in-place): the panel ROOT by
    // `Visibility`, the two SUB-BLOCKS by `Display` (None removes a hidden block from layout,
    // so the panel sizes to the visible block only — GTW-295).
    let cell = effective_cell(reads.target.effective());
    let occupant = cell.and_then(|c| reads.grid.as_deref().and_then(|g| g.occupant(&c)));

    // Resolve the two sub-block entities up front (immutable Entity reads) so the per-branch
    // `Display` writes do not re-borrow `nodes` while a marker query is still borrowed.
    let host = nodes.host.iter().next();
    let object_block = nodes.object_block.iter().next();

    if let Some(ganger) = occupant.and_then(|e| data.get(e).ok()) {
        // A hovered GANGER → show the panel + the ganger sub-block, hide the object block.
        toggle(&mut widgets.visibility, &nodes.root, Visibility::Inherited);
        set_display(&mut nodes.display, host, Display::Flex);
        set_display(&mut nodes.display, object_block, Display::None);
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
        set_display(&mut nodes.display, host, Display::None);
        set_display(&mut nodes.display, object_block, Display::Flex);
        // Clear the ganger block so it carries no stale data while hidden.
        clear_stat_block(refs, &mut widgets);
        fill_object_block(&mut widgets, &nodes, entry);
        return;
    }

    // Bare floor / nothing hovered → hide the whole panel.
    toggle(&mut widgets.visibility, &nodes.root, Visibility::Hidden);
}

/// Resolves the EFFECTIVE inspect [`InspectMode`] to the sim cell the panel describes (GTW-300).
///
/// - [`InspectMode::Hovered`]`(cell)` → that cell (the live cursor cell — `None` = bare floor /
///   off-map, so the panel hides). The no-pin case: the panel follows the cursor.
/// - [`InspectMode::Pinned`]`(cell)` → that PINNED cell, FROZEN against the cursor — the panel
///   keeps describing whatever occupies it (an enemy occupant OR a cover/wall) because the pin is
///   a CELL the same downstream occupant / terrain resolution already handles (no entity→cell
///   reverse lookup needed: a cell pin sidesteps the grid's missing reverse map entirely).
const fn effective_cell(mode: InspectMode) -> Option<gdtf_battle_sim::CellLevel> {
    match mode {
        InspectMode::Hovered(cell) => cell,
        // GTW-300 slice 3 — a pinned cell IS the cell the panel describes; the rest of
        // `update_inspect_panel` resolves it to an enemy / cover exactly like a hovered cell.
        InspectMode::Pinned(cell) => Some(cell),
    }
}

/// Sets the [`Node::display`] of `entity` (if present) to `want` via the `display` writer
/// (GTW-295: a hidden sub-block is `Display::None`, removed from layout so the panel sizes to
/// the visible block only).
///
/// Writes only when the display differs (change-detection hygiene); a missing entity / node is
/// a graceful no-op.
fn set_display(
    display: &mut Query<&mut Node, Without<ProgressBarFill>>,
    entity: Option<Entity>,
    want: Display,
) {
    if let Some(entity) = entity
        && let Ok(mut node) = display.get_mut(entity)
        && node.display != want
    {
        node.display = want;
    }
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

/// Fills the object block from a hovered [`CoverEntry`] (GTW-295 AC3) — a readable object
/// stat block comparable to the ganger block, mutate-in-place ([[ui-mutate-not-respawn]]):
///
/// - the TITLE line ([`InspectObjectText`]) → `"Cover"` (the object kind);
/// - the Integrity [`ProgressBar`] ([`InspectObjectBar`]) → `current_hp / max_hp`;
/// - the Hardness line ([`InspectObjectHardness`]) → `"Hardness {armor_hardness}"`;
/// - the Protection line ([`InspectObjectProtection`]) → `"Protection {armor_protection}"`;
/// - the Height-band line ([`InspectObjectHeight`]) → `"Height: {height_band}"`.
///
/// Each `Text` write is gated on a real change; the static "Integrity" label is spawned once
/// and never rewritten.
fn fill_object_block(widgets: &mut StatBlockWidgets, nodes: &InspectNodes, entry: CoverEntry) {
    set_line(widgets, nodes.object_title.iter().next(), "Cover");
    set_line(
        widgets,
        nodes.hardness.iter().next(),
        &format!("Hardness {}", *entry.armor_hardness),
    );
    set_line(
        widgets,
        nodes.protection.iter().next(),
        &format!("Protection {}", *entry.armor_protection),
    );
    set_line(
        widgets,
        nodes.height.iter().next(),
        &format!("Height: {}", height_label(entry.height_band)),
    );
    if let Some(bar) = nodes.object_bar.iter().next() {
        let fraction =
            FillFraction::from_ratio(hp_as_f32(*entry.current_hp), hp_as_f32(*entry.max_hp));
        set_progress_bar(bar, fraction, &widgets.children, &mut widgets.fills);
    }
}

/// Writes `value` into the `Text` of `entity` (if present) via the stat-block `texts` writer,
/// mutate-in-place and gated on a real change ([[ui-mutate-not-respawn]]).
fn set_line(widgets: &mut StatBlockWidgets, entity: Option<Entity>, value: &str) {
    if let Some(entity) = entity
        && let Ok(mut text) = widgets.texts.get_mut(entity)
        && text.as_str() != value
    {
        value.clone_into(&mut text.0);
    }
}

/// The display label for a cover's [`HeightBand`] (the LOW / MID / HIGH clearance band).
///
/// [`HeightBand`] has no `Display`, so the hover block names it directly here (a small,
/// fixed enum — no domain logic, just the on-screen caption).
const fn height_label(band: HeightBand) -> &'static str {
    match band {
        HeightBand::Low => "Low",
        HeightBand::Mid => "Mid",
        HeightBand::High => "High",
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
