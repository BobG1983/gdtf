//! Unit tests for the unified theme-definition model (GTW-485). Wiring only: `mod`
//! declarations, no test bodies.
//!
//! - [`parse`] — a theme RON literal (key uuid, `display_name`, `default_floor`
//!   terrain-uuid, terrain list of uuids) parses into a [`UuidThemeDef`](super::UuidThemeDef) (C1).
//! - [`round_trip`] — `deserialize(serialize(theme_def)) == theme_def`, identity only (C2).
//! - [`registry`] — insert-by-`ThemeUuid` + lookup + resolve `default_floor` + enumerate the
//!   terrain UUID list, through the REAL registry (C3).

mod parse;
mod registry;
mod round_trip;
