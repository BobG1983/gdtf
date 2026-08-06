//! Guard: the tracked `assets/` tree must carry no unstaged changes when the
//! suite runs, so a test that writes into shipped content is caught loudly.

mod check;
mod git;
