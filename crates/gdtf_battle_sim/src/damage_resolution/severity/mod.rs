mod kind;
mod roll;

pub use kind::{PartSeverityMod, Severity, SeverityRank, SeverityScore, part_severity_mod};
pub use roll::{SeverityInputs, roll_severity};

#[cfg(test)]
mod test;
