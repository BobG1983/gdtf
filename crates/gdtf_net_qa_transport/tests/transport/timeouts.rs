use gdtf_qa_protocol::timeouts::{DEFAULT_IO_TIMEOUT, DEFAULT_REPLY_TIMEOUT};

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
