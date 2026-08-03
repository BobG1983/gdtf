use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaError {
                        Malformed,
                                NotNegotiated,
            VersionMismatch,
        Busy,
                Timeout,
}
