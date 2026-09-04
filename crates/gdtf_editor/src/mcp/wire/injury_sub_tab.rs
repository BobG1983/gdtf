//! The Injury tab's sub-tabs on the wire.

use serde::{Deserialize, Serialize};

use crate::InjurySubTab;

/// Which record the Injury tab's central panel has open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum InjurySubTabNet {
    /// The injury def form.
    Def,
    /// The weighting table form.
    Tables,
}

impl InjurySubTabNet {
    /// Mirror the editor's own sub-tab, with no wildcard arm.
    pub(in crate::mcp) const fn from_sub_tab(sub_tab: InjurySubTab) -> Self {
        match sub_tab {
            InjurySubTab::Def => Self::Def,
            InjurySubTab::Tables => Self::Tables,
        }
    }

    /// Read a client's sub-tab back as the editor's own, with no wildcard arm.
    pub(in crate::mcp) const fn to_sub_tab(self) -> InjurySubTab {
        match self {
            Self::Def => InjurySubTab::Def,
            Self::Tables => InjurySubTab::Tables,
        }
    }
}
