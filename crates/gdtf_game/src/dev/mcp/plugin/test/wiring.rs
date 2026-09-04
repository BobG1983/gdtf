use bevy::prelude::*;
use cobalt_mcp_transport::NetInbox;

use crate::dev::mcp::plugin::mcp_plugin::McpPlugin;

#[test]
fn no_port_opens_no_listener() {
    let mut app = App::new();
    app.add_plugins(McpPlugin::from_port(None));

    assert!(
        app.world().get_resource::<NetInbox>().is_none(),
        "the inbox only reaches the world from the serve path, so its presence means a listener \
         thread started for a host that was given no port"
    );
}
