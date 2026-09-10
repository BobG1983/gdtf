//! Guard: one system in the game crate writes the battle-setup request, so the two routes
//! that used to generate a map cannot come back.

mod check;
mod scan;
mod tree;
