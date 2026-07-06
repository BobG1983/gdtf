//! The [`GdtfApp`] application wrapper — the ONE production entry point.
//!
//! Purely the wrapper (GTW-632): the DEV-ONLY QA affordances that used to live
//! beside it (auto-battle, capture, the drive triggers) are owned by `crate::dev`
//! and wired in through its one aggregate plugin.

mod gdtf_app;
pub use gdtf_app::GdtfApp;
