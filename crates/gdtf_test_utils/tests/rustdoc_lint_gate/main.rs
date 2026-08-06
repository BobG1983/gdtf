//! Cargo config no lint can catch: the workspace denies the whole rustdoc group, every member opts
//! in, and the content-editor bin skips the docs so it cannot overwrite the library crate's page.
mod check;
mod manifest;
mod tree;
