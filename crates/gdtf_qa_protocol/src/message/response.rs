use serde::{Deserialize, Serialize};

use super::{error::QaError, hello::HelloFacts};
use crate::command::{CommandCatalogue, CommandOutcome};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaResponse {
        HelloOk(HelloFacts),
            Catalogue(CommandCatalogue),
            Outcome(CommandOutcome),
        Error(QaError),
}
