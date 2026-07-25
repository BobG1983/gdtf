//! Spawning the Options screen (GTW-637, extended by GTW-868's dev-only setting row).
//!
//! Wiring only (module-layout rule 2). The concerns:
//!
//! - [`layout`] — the screen's spacing / sizing constants.
//! - [`toggle`] — the shared pill-toggle builder every setting row uses, so a new toggle
//!   is a call rather than a copy of the visual.
//! - [`row`] — the shared setting-row builder (caption + toggle + value readout).
//! - [`screen`] — the `OnEnter` system that builds the whole tree, and the `OnExit` system
//!   that clears the screen's navigation edges.
//! - [`stepper_row`] — the DEV-ONLY procgen-stepper setting row (`dev_tools` builds only).

mod layout;
mod row;
mod screen;
#[cfg(feature = "dev_tools")]
mod stepper_row;
mod toggle;

pub(in crate::states::running::options) use screen::{clear_options_nav_map, spawn_options_screen};
