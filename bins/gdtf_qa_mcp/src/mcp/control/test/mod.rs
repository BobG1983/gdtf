//! Unit tests for the three host-local tools: the recipe, port and line cap they read, the
//! link re-pointing, and what each outcome renders (GTW-745, GTW-875, GTW-808, GTW-943).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - `support` — the stub lifecycle, the recording link, and the directory fixture.
//! - `launch` — what a launch call reads and hands the lifecycle.
//! - `logs` — what a logs call reads, what it asks the lifecycle for, and what it renders.
//! - `render` — what each launch / stop outcome renders.

mod launch;
mod logs;
mod render;
mod support;
