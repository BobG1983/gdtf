//! The battle-lifecycle test suite, split by concern alongside the source modules:
//! shared fixtures in [`support`], plugin wiring in [`plugin`], the setup/teardown
//! drivers in [`setup`], the resources + message payloads in [`resources`], and the
//! roster-grounded outcome census in [`outcome`].

mod support;

mod outcome;
mod plugin;
mod resources;
mod setup;
