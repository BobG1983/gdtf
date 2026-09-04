use mcp::{ChildPid, OrphanPid, OrphanWatch, PortHold, QaPort, SystemOrphanWatch};

use super::{super::support::spawn_fake_game, placeholder::PROBE};

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
