//! Guard: every script under `.claude/workflows/` parses, and the build agent's output
//! schema stays small enough for a build to finish.

mod check;
mod parse;
mod schema;
mod tree;
