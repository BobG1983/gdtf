//! The link must wait longer than the host it is talking to.

use cobalt_mcp_protocol::timeouts::DEFAULT_REPLY_TIMEOUT;
use cobalt_mcp_server::link::LINK_TIMEOUT;

#[test]
fn the_link_outwaits_the_hosts_own_reply_wait() {
    assert!(
        *DEFAULT_REPLY_TIMEOUT < *LINK_TIMEOUT,
        "the host answers a command that ran out of time with `Timeout` after {:?}, so the link \
         must still be reading at that point — with a link timeout of {:?} the agent gets a \
         socket error instead of the host's own answer",
        *DEFAULT_REPLY_TIMEOUT,
        *LINK_TIMEOUT,
    );
}
