//! Cargo config no lint can catch: the workspace denies the rustdoc and rust warning groups, every
//! member opts in, and the content-editor bin's docs cannot overwrite the library crate's page.
mod check;
mod manifest;
mod tree;
