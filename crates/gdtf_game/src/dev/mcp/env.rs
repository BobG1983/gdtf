use std::ffi::{OsStr, OsString};

use cobalt_mcp_protocol::ports::{MCP_PORT_FLAG, McpPort};

/// The listen port a raw value names, or `None` when it is absent or unparsable.
#[must_use]
pub(super) fn port_from(raw: Option<&str>) -> Option<McpPort> {
    raw.and_then(|value| value.trim().parse::<u16>().ok())
        .map(McpPort::new)
}

// The listen port the argument after `--mcp-port` names, or `None` when there is none.
fn port_in(args: impl IntoIterator<Item = OsString>) -> Option<McpPort> {
    let mut from_flag = args
        .into_iter()
        .skip_while(|arg| arg.as_os_str() != OsStr::new(MCP_PORT_FLAG));
    port_from(from_flag.nth(1).as_deref().and_then(OsStr::to_str))
}

/// The listen port this process was started with, or `None` when it was given none.
#[must_use]
pub(super) fn port_from_args() -> Option<McpPort> {
    port_in(std::env::args_os())
}

#[cfg(test)]
mod test {
    use std::ffi::OsString;

    use cobalt_mcp_protocol::ports::McpPort;

    use super::{port_from, port_in};

    // An argument vector as the launcher hands one to the game binary.
    fn argv(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_parsable_value_is_the_listen_port() {
        assert_eq!(port_from(Some("7620")), Some(McpPort::new(7620)));
    }

    #[test]
    fn an_absent_value_opens_no_listener() {
        assert_eq!(port_from(None), None);
    }

    #[test]
    fn an_unparsable_value_opens_no_listener() {
        assert_eq!(port_from(Some("banana")), None);
    }

    #[test]
    fn the_argument_after_the_port_flag_is_the_listen_port() {
        assert_eq!(
            port_in(argv(&["game", "--mcp-port", "7620"])),
            Some(McpPort::new(7620))
        );
    }

    #[test]
    fn an_argument_vector_without_the_port_flag_opens_no_listener() {
        assert_eq!(port_in(argv(&["game", "--other", "7620"])), None);
        assert_eq!(port_in(argv(&["game", "--mcp-port"])), None);
    }

    #[test]
    fn an_unparsable_argument_opens_no_listener() {
        assert_eq!(port_in(argv(&["game", "--mcp-port", "banana"])), None);
    }
}
