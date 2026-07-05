//! Armor model proofs: the body-part keying, the newtype Deref, the
//! armor-type-carrying piece, the registry round-trip, and the dual-vocabulary
//! wheel parity. (The battle-local wear/`protects` gate is now proven on the
//! armor-piece ENTITIES — `crate::armor::wear_armor` on `&mut ArmorIntegrity` — by
//! `armor_wear`, `apply_hit`, and `situation::test::armor_pieces`, GTW-323.)

mod armor_type;
mod registry;
mod spec;
