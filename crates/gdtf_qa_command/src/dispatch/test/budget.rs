use gdtf_qa_protocol::timeouts::DEFAULT_REPLY_TIMEOUT;

use crate::dispatch::DEFERRED_BUDGET;

#[test]
fn the_deferral_budget_is_strictly_shorter_than_the_socket_reply_wait() {
    assert!(
        *DEFERRED_BUDGET < *DEFAULT_REPLY_TIMEOUT,
        "the deferral budget ({:?}) must expire before the transport stops waiting for the reply \
         ({:?})",
        *DEFERRED_BUDGET,
        *DEFAULT_REPLY_TIMEOUT
    );
}
