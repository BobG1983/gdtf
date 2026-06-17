//! The presenter plugin seam: the [`BattlePresenterPlugin`] mode selector and the two
//! renderer plugins (the real [`TopDownRendererPlugin`] and the [`IsoRendererPlugin`]
//! stub) it builds.

mod renderer;

#[cfg(test)]
mod test;

pub use renderer::{
    BattlePresenterMode, BattlePresenterPlugin, IsoRendererPlugin, TopDownRendererActive,
    TopDownRendererPlugin,
};
