use core::time::Duration;
use std::error::Error;

use cobalt_mcp_host::dispatch::{DeferredBudget, DeferredBudgetOverride};
use cobalt_mcp_protocol::timeouts::{NetIoTimeout, NetReplyTimeout, NetTimeouts};

pub(crate) type TestError = Box<dyn Error + Send + Sync>;

pub(crate) type TestResult = Result<(), TestError>;

/// A deferral budget no run reaches, so a parked reply waits out any load.
pub(crate) const NO_DEFERRAL_DEADLINE: DeferredBudgetOverride =
    DeferredBudgetOverride::new(DeferredBudget::new(Duration::MAX));

/// Socket waits no run reaches, so the listener never cuts a reply short.
pub(crate) const NO_SOCKET_DEADLINE: NetTimeouts = NetTimeouts::new(
    NetIoTimeout::new(Duration::MAX),
    NetReplyTimeout::new(Duration::MAX),
);
