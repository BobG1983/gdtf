//! What the real lookup names for a port a live listener holds.

use gdtf_qa_mcp::{ChildPid, OrphanPid, OrphanWatch, PortHold, QaPort, SystemOrphanWatch};

use super::{super::support::spawn_fake_game, placeholder::PROBE};

/// The real lookup names THIS process for a port this process bound — the fact the whole
/// adoption rests on. A machine with no `lsof` answers [`OrphanPid::Unknown`], the
/// report-only path, not a wrong pid.
#[test]
fn the_real_lookup_names_the_process_holding_the_port() {
    let port = QaPort::new(spawn_fake_game());
    let hold = SystemOrphanWatch::new().inspect(port, PROBE);
    let PortHold::Orphan(pid) = hold else {
        unreachable!("a live QA listener is an orphan to a manager that owns nothing");
    };
    assert!(
        matches!(pid, OrphanPid::Known(found) if found == ChildPid::new(std::process::id()))
            || pid == OrphanPid::Unknown,
        "the lookup names this process or names none, got: {pid:?}"
    );
}
