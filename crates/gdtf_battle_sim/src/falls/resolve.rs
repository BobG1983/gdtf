//! Find the landing level under a cell when upper support is gone.

use crate::{
    falls::StoreysFallen,
    metric::{Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

/// Where a drop ends and how far it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DropLanding {
    /// Level the ganger lands on.
    pub landing: Level,
    /// Storeys fallen.
    pub storeys: StoreysFallen,
}

/// Walk down from `start` until a supporting slab (or ground) is found.
#[must_use]
pub fn resolve_drop(cell: Cell, start: Level, surface: &SurfaceGrid) -> Option<DropLanding> {
    let start_z = *start;
    if start_z == 0 {
        return None;
    }

    let mut k = start_z - 1;
    loop {
        let supports = k == 0
            || surface.slab_state(&CellLevel::new(cell, Level::new(k))) == SlabState::Present;
        if supports {
            let landing = Level::new(k);
            let storeys = StoreysFallen::new(start_z - k);
            return Some(DropLanding { landing, storeys });
        }
        k -= 1;
    }
}
