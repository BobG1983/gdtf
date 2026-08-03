use bevy::{prelude::*, text::TextColor as UiTextColor, ui::Display};
use gdtf_battle_input::InspectMode;
use gdtf_battle_sim::prelude::Faction;
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

const ENEMY_TINT: Color = Color::srgb(0.86, 0.26, 0.22);

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

    let cell = effective_cell(reads.target.effective());
    let occupant = cell.and_then(|c| {
        let grid = reads.occupancy()?;
        let occupant = grid.occupant(&c)?;
        occupant_squad_visible(c, occupant, &factions, &reads).then_some(occupant)
    });

    let host = nodes.host.iter().next();
    let object_block = nodes.object_block.iter().next();

    if let Some(ganger) = occupant.and_then(|e| data.get(e).ok()) {
        toggle(&mut widgets.visibility, &nodes.root, Visibility::Inherited);
        set_display(&mut nodes.display, host, Display::Flex);
        set_display(&mut nodes.display, object_block, Display::None);
        tint_name(refs.name, *ganger.faction, &mut tint);
        update_stat_block(refs, &ganger, &mut widgets);
        return;
    }

    if let Some(entry) = cell.and_then(|c| object_entry(c, reads.occupancy(), reads.cover())) {
        toggle(&mut widgets.visibility, &nodes.root, Visibility::Inherited);
        set_display(&mut nodes.display, host, Display::None);
        set_display(&mut nodes.display, object_block, Display::Flex);
        clear_stat_block(refs, &mut widgets);
        fill_object_block(&mut widgets, &nodes, entry);
        return;
    }

    toggle(&mut widgets.visibility, &nodes.root, Visibility::Hidden);
}

const fn effective_cell(mode: InspectMode) -> Option<gdtf_battle_sim::metric::CellLevel> {
    match mode {
        InspectMode::Hovered(cell) => cell,
        InspectMode::Pinned(cell) => Some(cell),
    }
}

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
