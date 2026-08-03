use gdtf_net_qa_transport::DEFAULT_IO_TIMEOUT;

use crate::dispatch::DEFERRED_BUDGET;

#[test]
fn the_deferral_budget_is_strictly_shorter_than_the_socket_timeout() {
    assert!(
        *DEFERRED_BUDGET < *DEFAULT_IO_TIMEOUT,
        "the deferral budget ({:?}) must expire before the transport's socket timeout ({:?})",
        *DEFERRED_BUDGET,
        *DEFAULT_IO_TIMEOUT
    );
}
