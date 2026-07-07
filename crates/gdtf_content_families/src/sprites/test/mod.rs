//! Unit tests for the sprite-def model (GTW-663). Wiring only: `mod`
//! declarations, no test bodies.
//!
//! - [`parse`] — the ruled RON schema parses per shape (sheet-rect source,
//!   file source, optional facings map, optional animation), a missing
//!   required field fails loudly, and a def round-trips through serde.

mod parse;
