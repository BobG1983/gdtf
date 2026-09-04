use core::time::Duration;

use cobalt_mcp_protocol::ports::McpPort;

use crate::lifecycle::{
    manager::port::pick_port,
    orphan::{OrphanPid, OrphanStop, OrphanTarget, OrphanWatch, PortHold},
    values::ProbeTimeout,
};

const PROBE: ProbeTimeout = ProbeTimeout::new(Duration::from_millis(1));

const REQUESTED: McpPort = McpPort::new(45000);

struct EveryPortFree;

impl OrphanWatch for EveryPortFree {
    fn inspect(&self, _port: McpPort, _timeout: ProbeTimeout) -> PortHold {
        PortHold::Free
    }

    fn stop(&self, _target: OrphanTarget) -> OrphanStop {
        OrphanStop::Stopped
    }
}

struct OrphanOn(Vec<McpPort>);

impl OrphanWatch for OrphanOn {
    fn inspect(&self, port: McpPort, _timeout: ProbeTimeout) -> PortHold {
        if self.0.contains(&port) {
            return PortHold::Orphan(OrphanPid::Unknown);
        }
        PortHold::Free
    }

    fn stop(&self, _target: OrphanTarget) -> OrphanStop {
        OrphanStop::Stopped
    }
}

fn window_above(port: McpPort) -> Vec<McpPort> {
    (1..=64u16)
        .filter_map(|step| port.checked_add(step))
        .map(McpPort::new)
        .collect()
}

#[test]
fn a_port_no_record_holds_is_the_port_a_launch_asked_for() {
    let chosen = pick_port(REQUESTED, &[], &EveryPortFree, PROBE);

    assert_eq!(
        chosen,
        Some(REQUESTED),
        "with no record on the requested port the launch keeps it"
    );
}

#[test]
fn a_port_a_record_holds_sends_the_launch_somewhere_else() {
    let taken = vec![REQUESTED, McpPort::new(*REQUESTED + 1)];

    let chosen = pick_port(REQUESTED, &taken, &EveryPortFree, PROBE);

    let Some(chosen) = chosen else {
        unreachable!("a free port above the requested one was available");
    };
    assert_ne!(
        chosen, REQUESTED,
        "the requested port is recorded, so a new instance cannot take it"
    );
    assert!(
        !taken.contains(&chosen),
        "the chosen port is held by no record, chosen: {chosen:?}, taken: {taken:?}"
    );
}

#[test]
fn the_requested_port_is_never_probed_for_an_orphan() {
    let orphan_port = McpPort::new(*REQUESTED + 1);
    let orphans = OrphanOn(vec![orphan_port]);

    let chosen = pick_port(orphan_port, &[], &orphans, PROBE);

    assert_eq!(
        chosen,
        Some(orphan_port),
        "a port no record holds comes back whatever the orphan watch says about it, so the \
         launch itself still answers the orphan"
    );
}

#[test]
fn an_orphan_held_alternative_is_skipped() {
    let orphan_port = McpPort::new(*REQUESTED + 1);
    let orphans = OrphanOn(vec![orphan_port]);

    let chosen = pick_port(REQUESTED, &[REQUESTED], &orphans, PROBE);

    assert_ne!(
        chosen,
        Some(orphan_port),
        "a port an orphan answers on is not handed to a new instance"
    );
    assert!(chosen.is_some(), "a free port above it was available");
}

#[test]
fn an_exhausted_search_window_answers_no_port() {
    let orphans = OrphanOn(window_above(REQUESTED));

    let chosen = pick_port(REQUESTED, &[REQUESTED], &orphans, PROBE);

    assert_eq!(
        chosen, None,
        "with every port above the requested one held, there is nowhere to put the instance"
    );
}
