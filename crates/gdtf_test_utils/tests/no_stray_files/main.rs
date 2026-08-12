//! Guard: a suite run must leave no untracked files anywhere in the repo, so a
//! test that writes into the tree is caught instead of shipping a stray directory.

mod check;
mod git;
