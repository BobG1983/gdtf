//! Open an authored prefab: the rows a picker offers, and the load that paints one.

use core::cmp::Ordering;

use bevy::prelude::Deref;
use gdtf_battle_sim::level::{Prefab, PrefabRegistry, PrefabSpec, SpawnRole, UuidThemeRegistry};

use crate::{canvas::CurrentEditLevel, editor_map::EditorMap, session::MapEditorSession};

/// Where a spawn role sits in the picker's row order.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SpawnRoleOrder(u8);

/// The rank the picker sorts a spawn role by: player areas, then enemy, then fill.
#[must_use]
const fn role_order(role: SpawnRole) -> SpawnRoleOrder {
    match role {
        SpawnRole::Player => SpawnRoleOrder(0),
        SpawnRole::Enemy => SpawnRoleOrder(1),
        SpawnRole::Fill => SpawnRoleOrder(2),
    }
}

// Name, then extent, then theme, then role, so rows under different keys never collapse.
fn compare_rows(left: &Prefab, right: &Prefab) -> Ordering {
    let (this, that) = (left.spec(), right.spec());
    (**left.name())
        .cmp(&**right.name())
        .then_with(|| this.size.width().cmp(&that.size.width()))
        .then_with(|| this.size.height().cmp(&that.size.height()))
        .then_with(|| this.size.levels().cmp(&that.size.levels()))
        .then_with(|| (*this.theme).cmp(&*that.theme))
        .then_with(|| role_order(this.role).cmp(&role_order(that.role)))
}

// The name, the extent, the theme and the role, so two rows under one name still read apart.
fn row_label(prefab: &Prefab) -> String {
    let spec = prefab.spec();
    format!(
        "{}  {}x{}x{}  {}  {:?}",
        **prefab.name(),
        *spec.size.width(),
        *spec.size.height(),
        *spec.size.levels(),
        *spec.theme,
        spec.role,
    )
}

/// Every authored prefab as the (prefab, label) row the open picker draws.
///
/// Sorted by name, grid size, theme and role, so the order does not follow the registry's
/// own hash walk.
#[must_use]
pub fn prefab_candidates(registry: &PrefabRegistry) -> Vec<(&Prefab, String)> {
    let mut rows: Vec<&Prefab> = registry.iter().collect();
    rows.sort_by(|left, right| compare_rows(left, right));
    rows.into_iter()
        .map(|prefab| {
            let label = row_label(prefab);
            (prefab, label)
        })
        .collect()
}

/// Load an authored prefab onto the canvas: its extent and theme, its cells, its storey.
///
/// The map is replaced outright, so nothing the author had painted survives the open, and
/// every cell is painted against the spec's own extent rather than the session's old one.
pub fn open_prefab(
    session: &mut MapEditorSession,
    map: &mut EditorMap,
    edit_level: &mut CurrentEditLevel,
    themes: &UuidThemeRegistry,
    spec: &PrefabSpec,
) {
    session.set_grid_size(spec.size);
    session.select_theme(spec.theme, themes.default_floor(&spec.theme));
    *map = EditorMap::new();
    for entry in &spec.placements {
        map.paint_at(entry.at, entry.piece, entry.facing, spec.size);
    }
    *edit_level = edit_level.clamped(spec.size);
}
