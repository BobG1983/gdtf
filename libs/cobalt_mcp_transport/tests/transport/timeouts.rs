use cobalt_mcp_protocol::timeouts::{DEFAULT_IO_TIMEOUT, DEFAULT_REPLY_TIMEOUT, NetTimeouts};

#[test]
fn an_idle_client_gives_the_channel_back_sooner_than_a_working_one() {
    assert!(
        *DEFAULT_IO_TIMEOUT < *DEFAULT_REPLY_TIMEOUT,
        "the listener serves one connection at a time, so the socket timeout ({:?}) must reap an \
         idle client well before the reply wait ({:?}) — collapsing the two lets a half-open \
         client hold the channel for as long as a real request could",
        *DEFAULT_IO_TIMEOUT,
        *DEFAULT_REPLY_TIMEOUT,
    );
}

#[test]
fn an_in_process_test_listener_never_reaps_the_case_that_is_driving_it() {
    assert!(
        *NetTimeouts::NO_IDLE_REAP.io() > *DEFAULT_REPLY_TIMEOUT,
        "a case drives the app's frames itself, so it reads a reply over as many frames as the \
         reply is long and sends nothing meanwhile. Its listener's read wait ({:?}) must outlast \
         the reply wait ({:?}), or the reap closes a connection the case is still using and the \
         next request comes back ConnectionReset",
        *NetTimeouts::NO_IDLE_REAP.io(),
        *DEFAULT_REPLY_TIMEOUT,
    );
}
