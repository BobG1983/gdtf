//! The editor's request drain (GTW-804, cut to the command layer by GTW-943).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`route`] — the drain itself.

pub(in crate::net_qa) mod route;

pub(in crate::net_qa) use route::route_editor_requests;
