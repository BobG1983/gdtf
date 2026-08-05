use bevy::prelude::*;
use gdtf_battle_sim::cover::{CoverEntry, HeightBand};
use gdtf_ui::{FillFraction, set_progress_bar};

use super::params::InspectNodes;
use crate::states::running::game::battlescape::stat_block::StatBlockWidgets;

pub(super) fn fill_object_block(
    widgets: &mut StatBlockWidgets,
    nodes: &InspectNodes,
    entry: CoverEntry,
) {
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

fn set_line(widgets: &mut StatBlockWidgets, entity: Option<Entity>, value: &str) {
    if let Some(entity) = entity
        && let Ok(mut text) = widgets.texts.get_mut(entity)
        && text.as_str() != value
    {
        value.clone_into(&mut text.0);
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
