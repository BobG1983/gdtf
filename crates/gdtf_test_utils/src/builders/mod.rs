//! Higher-level, one-call app builders that compose the lower-level
//! [`GdtfTestAppBuilder`](crate::GdtfTestAppBuilder) drive into a ready-to-assert
//! app for a specific scenario.

mod battle_app;

pub use battle_app::BattleAppBuilder;
