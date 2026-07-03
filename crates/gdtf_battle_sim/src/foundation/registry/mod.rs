//! The generic [`Registry`] catalog map (GTW-567) — the ONE `key → definition`
//! surface the per-family registry [`Resource`](bevy::prelude::Resource)
//! newtypes wrap and delegate to.

mod map;
#[cfg(test)]
mod test;

pub use map::Registry;
