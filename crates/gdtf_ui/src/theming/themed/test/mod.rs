//! Tests for the central `apply_theme` base-look pass, split by concern:
//! shared fixtures, per-role painting, live retheme, and the GTW-284
//! incremental repaint.

mod support;

mod incremental;
mod paint_roles;
mod retheme;
