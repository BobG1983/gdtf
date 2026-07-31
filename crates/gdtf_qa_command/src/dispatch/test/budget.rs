//! The deferral budget expires before the socket does.

use gdtf_net_qa_transport::DEFAULT_IO_TIMEOUT;

use crate::dispatch::DEFERRED_BUDGET;

/// A deferred reply that never settles must produce an ANSWER, not a dead socket.
///
/// The inequality is what makes that true, and it lives across two crates, so it is pinned
/// here rather than left to the two numbers staying in step by luck.
#[test]
fn the_deferral_budget_is_strictly_shorter_than_the_socket_timeout() {
    assert!(
        *DEFERRED_BUDGET < *DEFAULT_IO_TIMEOUT,
        "the deferral budget ({:?}) must expire before the transport's socket timeout ({:?})",
        *DEFERRED_BUDGET,
        *DEFAULT_IO_TIMEOUT
    );
}
