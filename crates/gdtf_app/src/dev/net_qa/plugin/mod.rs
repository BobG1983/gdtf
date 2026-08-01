//! [`NetQaPlugin`] — the DEV-ONLY QA network control channel's registration
//! (GTW-736; split into per-concern files per module-layout when GTW-802 added the
//! focus-drive consumer).
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **`cfg(all(debug_assertions, feature = "net_qa"))`.** The wiring site
//!    ([`crate::dev::plugin`]) only adds the plugin under a debug build AND the opt-in
//!    `net_qa` feature — it opens a listener, so a release artifact never sees it, even
//!    with the feature on.
//! 2. **Opt-in env var.** Even in a `net_qa` debug build the plugin is INERT by default:
//!    [`NetQaPlugin::from_env`] reads `GDTF_NET_QA` and registers NOTHING unless it is set
//!    truthy, so a normal `cargo run` reaches a battle exactly as before.
//!
//! ## Members (one concern per file)
//!
//! - [`net_qa_plugin`] — the [`NetQaPlugin`] type, its constructors, and its `Plugin` impl.
//! - [`register_transport`] — the capture queue, the pump's cross-frame state, the router
//!   and the game's command set.
//! - [`register_consumers`] — the per-consumer system registrations and their ordering.

mod net_qa_plugin;
mod register_consumers;
mod register_transport;

crate::support_use!(net_qa_plugin::NetQaPlugin;);
