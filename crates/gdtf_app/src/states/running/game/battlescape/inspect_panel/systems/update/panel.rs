//! The inspect-panel repaint driver: effective-target resolution, sub-block toggling,
//! and the faction name tint. Split out of the monolithic `update.rs` (GTW-583); the
//! repaint rationale lives on the parent `update` module.

use bevy::{prelude::*, text::TextColor as UiTextColor, ui::Display};
use gdtf_battle_input::InspectMode;
use gdtf_battle_sim::Faction;
use gdtf_ui::ProgressBarFill;

use super::{
    fog::occupant_squad_visible,
    object_block::{fill_object_block, object_entry},
    params::{FactionTint, InspectNodes, InspectReads},
};
use crate::states::running::game::battlescape::{
    inspect_panel::components::InspectStatBlockHost,
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

/// Repaints the inspect panel from the current EFFECTIVE [`InspectTarget`](gdtf_battle_input::InspectTarget) (pinned-else-hovered,
/// GTW-300): a pinned cell freezes the panel on its occupant / cover; with no pin it follows the
/// live hovered cell.
///
/// Resolves the effective cell to a ganger (the grid's
/// [`occupant`](gdtf_battle_sim::OccupancyGrid::occupant) with [`StatBlockData`]), else a
/// non-floor object (the grid's [`terrain`](gdtf_battle_sim::OccupancyGrid::terrain) being a
/// wall / cover, with its seeded [`CoverEntry`](gdtf_battle_sim::CoverEntry) from the ledger), else bare floor; and toggles
/// the panel + its two sub-blocks accordingly. On a ganger hover it ALSO tints the name line
/// by the ganger's [`Faction`] (AC2 — enemy red-ish, player the normal theme color). Param-only
/// (`bevy-traps.md` #7).
pub(in crate::states::running::game::battlescape) fn update_inspect_panel(
    reads: InspectReads,
    blocks: Query<&StatBlockRefs, With<InspectStatBlockHost>>,
    data: Query<StatBlockData>,
    factions: Query<&Faction>,
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
    // GTW-378 — a hovered occupant feeds the panel ONLY when its cell is currently
    // squad-VISIBLE (the fog gate), so a fog-hidden enemy never populates the inspect panel
    // (the info-leak that quietly revealed where the fog hides an enemy). A non-visible
    // occupant resolves to `None`, falling through to the object / floor branches below — a
    // BLOCKING wall / cover still inspects (it is map geometry / mission memory, not a hidden
    // enemy). FAIL-CLOSED: an absent fog treats every occupant as non-visible.
    let occupant = cell.and_then(|c| {
        let grid = reads.grid.as_deref()?;
        let occupant = grid.occupant(&c)?;
        occupant_squad_visible(c, occupant, &factions, &reads).then_some(occupant)
    });

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
/// a live battle, `bevy-traps.md` #1). With no [`PlayerFaction`](gdtf_battle_sim::PlayerFaction) resource the tint falls back
/// to the normal theme color (no allegiance known → no enemy highlight); with no
/// [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) the player color falls back to the bevy default text color. The hover update
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
