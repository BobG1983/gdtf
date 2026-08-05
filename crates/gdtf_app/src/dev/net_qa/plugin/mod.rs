//! 1. **`cfg(debug_assertions)`.** The wiring site
mod net_qa_plugin;
mod register_consumers;
mod register_present;
mod register_transport;

crate::support_use!(net_qa_plugin::NetQaPlugin;);
