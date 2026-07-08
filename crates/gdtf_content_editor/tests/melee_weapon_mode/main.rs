//! GTW-671 C4/C5: the MELEE mode's REAL round-trips through the actual
//! `MeleeWeaponsFamily` loader from `TempDir` assets roots (the shipped `assets/` tree
//! is NEVER written), plus the FISTS guard over the shipped default's shape.
//!
//! Dir-form suite (module-layout rule 5): the shared headless-app recipe lives in
//! [`harness`]; the C4 maximal + minimal save→reload round-trips live in
//! [`roundtrip`]; the C5 fists no-drift guard lives in [`fists`].

mod fists;
mod harness;
mod roundtrip;
