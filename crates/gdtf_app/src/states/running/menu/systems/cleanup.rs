use bevy::{input_focus::directional_navigation::DirectionalNavigationMap, prelude::*};

pub(in crate::states::running::menu) fn clear_nav_map(
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    nav_map.clear();
}
