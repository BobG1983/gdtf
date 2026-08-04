//! Fire orders: volleys, blasts, and the ECS queries that feed them.

mod blast;
mod compose;
mod query;
mod volley;

pub use blast::{BlastFootprint, resolve_blast};
pub use query::{
    BattleGrids, FireOrder, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, StruckBodies,
    TargetQuery, WeaponQuery, WearsQuery, WieldedWeapons, WieldsQuery,
};
pub use volley::{Volley, fire};

#[cfg(test)]
mod test;
