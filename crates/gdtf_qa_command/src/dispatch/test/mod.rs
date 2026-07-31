//! Unit tests for the dispatch path.
//!
//! - [`claim`] — the inbox's per-name take and the decode step's two outcomes.
//! - [`budget`] — the deferral budget sits strictly inside the transport's socket timeout.
//! - [`call_debug`] — a queued call prints its name and its decoded arguments.
//! - [`responder`] — the typed responder's three answers.
//! - [`change_detection`] — an idle frame dirties neither shared resource.
//! - [`ordering`] — registering a command chains Route before Claim itself.
//! - [`parking`] — answering one parked call at a time.

mod budget;
mod call_debug;
mod change_detection;
mod claim;
mod ordering;
mod parking;
mod responder;
