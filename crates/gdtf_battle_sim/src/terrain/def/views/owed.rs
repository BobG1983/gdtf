//! Which views a terrain def owes, read off its kind and its tags.

use bevy::prelude::Deref;

use super::art::TerrainView;
use crate::terrain::{
    def::{TerrainDef, TerrainTag},
    entity::TerrainPieceKind,
    facing::{TerrainCorner, TerrainFacing},
};

/// The views one terrain def owes art for.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct OwedViews(Vec<TerrainView>);

impl OwedViews {
    /// Wrap the derived views.
    #[must_use]
    pub const fn new(views: Vec<TerrainView>) -> Self {
        Self(views)
    }
}

// A door owes its shut and its open drawing on every side.
fn door_views() -> Vec<TerrainView> {
    TerrainFacing::ALL
        .into_iter()
        .flat_map(|facing| [TerrainView::Shut(facing), TerrainView::Open(facing)])
        .collect()
}

// A staircase owes the climb seen from below and from above on every side.
fn stair_views() -> Vec<TerrainView> {
    TerrainFacing::ALL
        .into_iter()
        .flat_map(|facing| {
            [
                TerrainView::FromBelow(facing),
                TerrainView::FromAbove(facing),
            ]
        })
        .collect()
}

// A wall owes its straight runs and the turns between them.
fn wall_views() -> Vec<TerrainView> {
    TerrainFacing::ALL
        .into_iter()
        .map(TerrainView::Edge)
        .chain(TerrainCorner::ALL.into_iter().map(TerrainView::Corner))
        .collect()
}

// Cover and an emplacement are drawn from whichever side they are seen.
fn faced_views() -> Vec<TerrainView> {
    TerrainFacing::ALL
        .into_iter()
        .map(TerrainView::Facing)
        .collect()
}

/// The views a piece of this kind, carrying these tags, owes art for.
/// The tags are read before the kind, and `Openable` before `Stair`.
#[must_use]
pub fn owed_views_for(kind: TerrainPieceKind, tags: &[TerrainTag]) -> OwedViews {
    if tags.contains(&TerrainTag::Openable) {
        return OwedViews::new(door_views());
    }
    if tags.contains(&TerrainTag::Stair) {
        return OwedViews::new(stair_views());
    }
    OwedViews::new(match kind {
        TerrainPieceKind::Wall => wall_views(),
        TerrainPieceKind::Cover | TerrainPieceKind::Emplacement => faced_views(),
        TerrainPieceKind::Slab => vec![TerrainView::Single],
    })
}

/// The views this def owes art for.
#[must_use]
pub fn owed_views(def: &TerrainDef) -> OwedViews {
    owed_views_for(def.sim_kind.kind(), &def.tags)
}
