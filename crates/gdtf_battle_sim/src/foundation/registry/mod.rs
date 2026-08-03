//! Shared key→value registry used by content families.

mod map;
#[cfg(test)]
mod test;

pub use map::Registry;
