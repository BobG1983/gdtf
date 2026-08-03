use crate::ganger::{Aiming, Facing, Position, Stance, Suppressed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shooter<'a> {
            pub stance:     &'a Stance,
            pub aiming:     &'a Aiming,
        pub position:   &'a Position,
        pub facing:     &'a Facing,
                            pub suppressed: Option<&'a Suppressed>,
}
