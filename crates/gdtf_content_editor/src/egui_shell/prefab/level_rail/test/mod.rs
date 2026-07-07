//! In-crate tests for the GTW-595 level rail's MODEL half (A1): the pure CPU occupancy
//! sweep ([`sweep`]), the change-keyed thumbnail cache ([`cache_keying`]), and the
//! click→level / scrub mapping ([`nav_mapping`]) — shared fixtures in [`support`]. The
//! egui draw itself is screenshot-QA'd (A2): egui closures never run headless.

mod cache_keying;
mod nav_mapping;
mod support;
mod sweep;
