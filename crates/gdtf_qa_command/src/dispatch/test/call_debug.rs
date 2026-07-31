//! A queued call prints its command name AND its decoded arguments.
//!
//! This is the whole reason [`QaCommand::Args`] carries a `Debug` bound and the reason
//! [`CommandCall`]'s `Debug` is hand-written rather than derived. The transport's deadline
//! sweep logs a timed-out entry as `debug!(request = ?entry.payload, …)`
//! (`gdtf_net_qa_transport`'s `PendingQueue::sweep_expired`), so the payload's `Debug` IS
//! the whole record of what was abandoned. Print only the name and a timeout tells you
//! which command hung but never with what — which is the half that lets you reproduce it.

use gdtf_qa_protocol::ids::{CellNet, CellXNet, CellYNet};

use crate::{
    command::QaCommand,
    dispatch::CommandCall,
    test_support::{FakeCell, FakeCellArgs, FakePhase, FakePhaseArgs},
};

/// A call prints its command's name and every argument field it was decoded from.
#[test]
fn a_queued_call_prints_its_name_and_its_arguments() {
    let call = CommandCall::<FakeCell>::new(FakeCellArgs {
        cell: CellNet::new(CellXNet::new(3), CellYNet::new(-4)),
    });

    let printed = format!("{call:?}");

    assert!(
        printed.contains(FakeCell::NAME.as_str()),
        "the log line must name the command that hung: {printed}"
    );
    assert!(
        printed.contains('3') && printed.contains("-4"),
        "the log line must carry the decoded arguments, not just the name: {printed}"
    );
}

/// An argument-less command still prints its name, so the two halves are independent.
#[test]
fn an_argument_less_call_still_prints_its_name() {
    let printed = format!("{:?}", CommandCall::<FakePhase>::new(FakePhaseArgs {}));

    assert!(
        printed.contains(FakePhase::NAME.as_str()),
        "the log line must name the command that hung: {printed}"
    );
}
