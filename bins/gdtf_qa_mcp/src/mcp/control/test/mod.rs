//! Unit tests for the four host-local tools: the recipe and port they read, the link
//! re-pointing, and what each outcome renders (GTW-745, GTW-875, GTW-808).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - `support` — the stub lifecycle, the recording link, and the directory fixture.
//! - `launch` — what a launch call reads and hands the lifecycle.
//! - `render` — what each launch / stop outcome renders.

mod launch;
mod render;
mod support;
