use bevy::prelude::Entity;

use crate::{
    cover::{CoverEntry, HeightBand},
    metric::{CellLevel, SimPos},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarchKind {
            Ganger(Entity),
            Cover(CoverEntry),
        Slab,
            Ground,
            Miss,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarchResult {
        pub kind:   MarchKind,
            pub at:     CellLevel,
            pub band:   HeightBand,
            pub impact: SimPos,
}
