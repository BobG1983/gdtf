use bevy::{
    input_focus::directional_navigation::DirectionalNavigationMap, math::CompassOctant, prelude::*,
};
use gdtf_battle_input::PanelNavOrder;
use gdtf_ui::DisabledButton;

pub(in crate::states::running::game::battlescape) const ACTION_BAR_NAV_BASE: u16 = 0;

pub(in crate::states::running::game::battlescape) const WEAPON_NAV_BASE: u16 = 100;

pub(in crate::states::running::game::battlescape) const CONTEXTUAL_NAV_BASE: u16 = 200;

pub(in crate::states::running::game::battlescape) fn rebuild_panel_nav_topology(
    mut nav_map: ResMut<DirectionalNavigationMap>,
    buttons: Query<(Entity, &PanelNavOrder, &Visibility, Has<DisabledButton>)>,
) {
    let mut chain: Vec<(PanelNavOrder, Entity)> = buttons
        .iter()
        .filter(|(_, _, visibility, disabled)| {
            !matches!(**visibility, Visibility::Hidden) && !disabled
        })
        .map(|(entity, order, ..)| (*order, entity))
        .collect();
    chain.sort_by_key(|(order, entity)| (*order, *entity));
    let ordered: Vec<Entity> = chain.into_iter().map(|(_, entity)| entity).collect();

    nav_map.clear();
    if ordered.len() >= 2 {
        nav_map.add_edges(&ordered, CompassOctant::East);
    }
}

pub(in crate::states::running::game::battlescape) fn clear_panel_nav_topology(
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    nav_map.clear();
}
