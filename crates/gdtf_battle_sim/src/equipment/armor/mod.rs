//! Body armor: pieces, specs, registry, and wear relationships.

mod registry;
mod relationship;
mod spec;
mod stats;
mod worn;

#[cfg(test)]
mod test;

pub use registry::{ArmorName, ArmorRegistry};
pub use relationship::{PieceArmorMut, Wears, WornArmor, WornBy};
pub use spec::ArmorSpec;
pub use stats::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType, BodyPart,
    InjuryCategory,
};
pub use worn::{SourceArmor, SourceArmorDef};
