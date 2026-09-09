//! The editor's MCP identity, its hello facts, and the resolution of the
//! port its listener binds.

use std::ffi::{OsStr, OsString};

use cobalt_mcp_protocol::{
    message::{HelloFacts, ProtocolVersion, ServerNameNet},
    ports::{MCP_PORT_FLAG, McpPort},
};

pub(super) const EDITOR_MCP_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;

/// Server name advertised on the editor MCP hello.
pub const EDITOR_MCP_SERVER_NAME: &str = "gdtf-editor-mcp";

#[must_use]
pub(super) fn editor_host_name() -> ServerNameNet {
    ServerNameNet::new(EDITOR_MCP_SERVER_NAME.to_owned())
}

#[must_use]
pub(super) fn editor_hello_facts() -> HelloFacts {
    HelloFacts::new(EDITOR_MCP_PROTOCOL_VERSION, editor_host_name())
}

#[must_use]
pub(super) fn editor_port_from(raw: Option<&str>) -> Option<McpPort> {
    raw.and_then(|value| value.trim().parse::<u16>().ok())
        .map(McpPort::new)
}

// The listen port the argument after `--mcp-port` names, or `None` when there is none.
fn editor_port_in(args: impl IntoIterator<Item = OsString>) -> Option<McpPort> {
    let mut from_flag = args
        .into_iter()
        .skip_while(|arg| arg.as_os_str() != OsStr::new(MCP_PORT_FLAG));
    editor_port_from(from_flag.nth(1).as_deref().and_then(OsStr::to_str))
}

#[must_use]
pub(super) fn editor_port_from_args() -> Option<McpPort> {
    editor_port_in(std::env::args_os())
}

#[cfg(test)]
mod test {
    use std::ffi::OsString;

    use cobalt_mcp_protocol::ports::McpPort;

    use super::{editor_port_from, editor_port_in};

    // An argument vector as the launcher hands one to the editor binary.
    fn argv(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_parsable_value_is_the_listen_port() {
        assert_eq!(editor_port_from(Some("7620")), Some(McpPort::new(7620)));
    }

    #[test]
    fn an_absent_value_opens_no_listener() {
        assert_eq!(editor_port_from(None), None);
    }

    #[test]
    fn an_unparsable_value_opens_no_listener() {
        assert_eq!(editor_port_from(Some("banana")), None);
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_parsing() {
        assert_eq!(editor_port_from(Some(" 7620 ")), Some(McpPort::new(7620)));
    }

    #[test]
    fn the_argument_after_the_port_flag_is_the_listen_port() {
        assert_eq!(
            editor_port_in(argv(&["editor", "--mcp-port", "7620"])),
            Some(McpPort::new(7620))
        );
    }

    #[test]
    fn an_argument_vector_without_the_port_flag_opens_no_listener() {
        assert_eq!(editor_port_in(argv(&["editor", "--other", "7620"])), None);
        assert_eq!(editor_port_in(argv(&["editor", "--mcp-port"])), None);
    }

    #[test]
    fn an_unparsable_argument_opens_no_listener() {
        assert_eq!(
            editor_port_in(argv(&["editor", "--mcp-port", "banana"])),
            None
        );
    }
}
