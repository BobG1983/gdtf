use bevy::prelude::Deref;

use crate::{
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adjacent8(bool);

impl Adjacent8 {
        #[must_use]
    pub const fn new(adjacent: bool) -> Self {
        Self(adjacent)
    }
}

#[must_use]
pub fn is_8_adjacent(a: Position, b: Position) -> Adjacent8 {
    let pa = **a;
    let pb = **b;
    if pa.z != pb.z {
        return Adjacent8::new(false);
    }
    let dx = (pa.x - pb.x).abs();
    let dy = (pa.y - pb.y).abs();
    Adjacent8::new(dx <= 1 && dy <= 1 && (dx != 0 || dy != 0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
        pub pos:     Position,
        pub life:    LifeState,
            pub faction: Faction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownedTarget {
        pub pos:          Position,
            pub life:         LifeState,
        pub faction:      Faction,
                        pub bleeding_out: Option<BleedingOut>,
}
