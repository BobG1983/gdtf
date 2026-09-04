use bevy::prelude::*;
use gdtf_battle_sim::{cover::HeightBand, emplacement::EmplacementState, entity::TerrainPieceKind};
use gdtf_ui::{FillFraction, set_progress_bar};

use super::params::InspectNodes;
use crate::states::running::game::battlescape::{
    inspect_panel::decide::InspectTerrain, stat_block::StatBlockWidgets,
};

pub(super) fn fill_object_block(
    widgets: &mut StatBlockWidgets,
    nodes: &InspectNodes,
    terrain: &InspectTerrain,
) {
    let cover = terrain.cover();
    set_line(widgets, nodes.object_title.iter().next(), &title(terrain));
    set_line(
        widgets,
        nodes.hardness.iter().next(),
        &cover.map_or_else(String::new, |entry| {
            format!("Hardness {}", *entry.armor_hardness)
        }),
    );
    set_line(
        widgets,
        nodes.protection.iter().next(),
        &cover.map_or_else(String::new, |entry| {
            format!("Protection {}", *entry.armor_protection)
        }),
    );
    set_line(
        widgets,
        nodes.height.iter().next(),
        &cover.map_or_else(String::new, |entry| {
            format!("Height: {}", height_label(entry.height_band))
        }),
    );
    if let Some(bar) = nodes.object_bar.iter().next() {
        let fraction = cover.map_or_else(
            || FillFraction::new(0.0),
            |entry| {
                FillFraction::from_ratio(hp_as_f32(*entry.current_hp), hp_as_f32(*entry.max_hp))
            },
        );
        set_progress_bar(bar, fraction, &widgets.children, &mut widgets.fills);
    }
}

// The piece kind, and for an emplacement the seat's state and the weapon it mounts.
fn title(terrain: &InspectTerrain) -> String {
    let kind = kind_label(terrain.kind());
    match terrain.emplacement() {
        Some(seat) => format!(
            "{kind} ({}, {})",
            manned_label(seat.state()),
            ***seat.weapon()
        ),
        None => kind.to_owned(),
    }
}

fn set_line(widgets: &mut StatBlockWidgets, entity: Option<Entity>, value: &str) {
    if let Some(entity) = entity
        && let Ok(mut text) = widgets.texts.get_mut(entity)
        && text.as_str() != value
    {
        value.clone_into(&mut text.0);
    }
}

const fn kind_label(kind: TerrainPieceKind) -> &'static str {
    match kind {
        TerrainPieceKind::Wall => "Wall",
        TerrainPieceKind::Cover => "Cover",
        TerrainPieceKind::Slab => "Slab",
        TerrainPieceKind::Emplacement => "Emplacement",
    }
}

const fn manned_label(state: EmplacementState) -> &'static str {
    match state {
        EmplacementState::Vacant => "Vacant",
        EmplacementState::Occupied => "Manned",
    }
}

const fn height_label(band: HeightBand) -> &'static str {
    match band {
        HeightBand::Low => "Low",
        HeightBand::Mid => "Mid",
        HeightBand::High => "High",
    }
}

fn hp_as_f32(hp: u32) -> f32 {
    f32::from(u16::try_from(hp).unwrap_or(u16::MAX))
}
