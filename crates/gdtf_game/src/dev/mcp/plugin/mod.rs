//! Wiring for the game MCP control channel: the plugin and its registration steps.
mod mcp_plugin;
mod register_consumers;
mod register_present;
mod register_transport;

#[cfg(test)]
mod test;

crate::support_use!(mcp_plugin::McpPlugin;);
