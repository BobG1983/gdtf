//! The GAME host's READ commands — one file per command, each answering from the world
//! without changing it (GTW-942).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`app_phase`] — `app.phase`, the five-level state read.

pub(crate) mod app_phase;

pub(crate) use app_phase::AppPhase;
