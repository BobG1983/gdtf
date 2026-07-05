//! The presenter plugin seam: the [`BattlePresenterPlugin`] mode selector and the two
//! renderer plugins (the real [`TopDownRendererPlugin`] and the [`IsoRendererPlugin`]
//! stub) it builds.

mod mode;
mod topdown;

#[cfg(test)]
mod test;

pub use mode::{BattlePresenterMode, BattlePresenterPlugin, IsoRendererPlugin};
pub use topdown::{TopDownRendererActive, TopDownRendererPlugin};
