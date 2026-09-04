//! What the control tools may ask of one host's children.

use cobalt_mcp_protocol::ports::McpPort;

use crate::lifecycle::{
    launch::{LaunchSpec, WorkingDir},
    outcome::{LaunchOutcome, StopOutcome},
    values::{InstanceId, OutputTail, RecordedInstance, TailLines},
};

/// Launch and stop the children of one host.
pub trait HostLifecycle {
    /// Spawn (or reuse) a host on `port` with `spec`.
    fn launch(&mut self, port: McpPort, spec: &LaunchSpec) -> LaunchOutcome;

    /// Stop the managed child or any orphan on `port`.
    fn stop(&mut self, port: McpPort) -> StopOutcome;

    /// Stop the instance with this id, leaving the other records alone.
    fn stop_instance(&mut self, instance: &InstanceId) -> StopOutcome;

    /// Stop every child this manager owns.
    fn stop_owned(&mut self) -> StopOutcome;

    /// Drop the records whose processes are no longer running.
    fn reap_dead_child(&mut self);

    /// Recorded instances, in launch order.
    fn instances(&self) -> Vec<RecordedInstance>;

    /// Working directory of the last recorded child, if any.
    fn child_working_dir(&self) -> Option<WorkingDir>;

    /// Working directory of one recorded instance, if it is recorded.
    fn instance_working_dir(&self, instance: &InstanceId) -> Option<WorkingDir>;

    /// Recent output of the last recorded child, if any.
    fn child_output(&self, max: TailLines) -> Option<OutputTail>;

    /// Recent output of one recorded instance, if it is recorded.
    fn instance_output(&self, instance: &InstanceId, max: TailLines) -> Option<OutputTail>;
}
