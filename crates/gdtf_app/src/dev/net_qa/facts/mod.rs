//! The GAME host's per-frame facts — the one thing every command's availability predicate
//! keys on, and the one read of the world that produces it (GTW-942).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`game_facts`] — [`GameFacts`], the plain value a predicate takes.
//! - [`read`] — [`GameFactsParam`], the `SystemParam` that samples it once a frame.

pub(crate) mod game_facts;
pub(crate) mod read;

pub(crate) use game_facts::GameFacts;
pub(crate) use read::GameFactsParam;
