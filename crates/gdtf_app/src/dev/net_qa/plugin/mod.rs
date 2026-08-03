//! 1. **`cfg(all(debug_assertions, feature = "net_qa"))`.** The wiring site
mod net_qa_plugin;
mod register_consumers;
mod register_transport;

crate::support_use!(net_qa_plugin::NetQaPlugin;);
