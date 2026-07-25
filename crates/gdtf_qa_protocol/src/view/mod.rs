//! The curated read-model DTOs — the wire snapshot a QA client reads (GTW-734).
//!
//! Independent serde types designed FROM what the sim exposes (ganger vitals, terrain
//! state, squad fog, app flow) but never a LEAK of a sim type — the crate is bevy-free.
//! One concern per file: the ganger stat scalars ([`stat`]), the life-state mirror
//! ([`life`]), the injury summary ([`injury`]), the weapon + indexed fire-mode list
//! ([`weapon`]), the [`ganger`] card, the [`terrain`] summary (with the door /
//! emplacement token handout), the [`panel`] button token handout, the [`menu`]
//! enumeration handout, the [`focus`] focusable-control enumeration handout, the [`fog`]
//! view, the [`appflow`] view, the [`selection`] view,
//! the top-level [`battle`] aggregate, and the [`editor`] query family (the content
//! editor's own per-topic read surface — ADR 0007).

pub mod appflow;
pub mod battle;
pub mod editor;
pub mod focus;
pub mod fog;
pub mod ganger;
pub mod injury;
pub mod life;
pub mod menu;
pub mod panel;
pub mod selection;
pub mod stat;
pub mod terrain;
pub mod weapon;

pub use appflow::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, RequestKindNet};
pub use battle::{BattleView, TurnView};
pub use editor::{
    EditorDraftFieldNameNet, EditorDraftFieldValueNet, EditorDraftFieldView, EditorDraftView,
    EditorFindingDetailNet, EditorFindingKindNet, EditorFindingSubjectNet, EditorFindingView,
    EditorModeLabelNet, EditorModeNet, EditorModeView, EditorQueryKind, EditorQueryOptionsView,
    EditorQueryReply, EditorQueryTopicView, EditorQueryView, EditorReadinessNet, EditorSessionView,
    EditorTabIndexNet, EditorTerrainKeyNet, EditorThemeKeyNet, EditorTopicDescriptionNet,
    EditorValidationView, ValidationChecksCompleteNet,
};
pub use focus::{
    FocusView, FocusableCheckedNet, FocusableEnabledNet, FocusableKindNet, FocusableLabelNet,
    FocusableView, FocusedNet,
};
pub use fog::{ExploredCellCountNet, FogView, VisibleCellCountNet};
pub use ganger::GangerView;
pub use injury::{BodyPartNet, InjuryEntryNet, InjuryNameNet, InjurySummaryNet, SeverityNet};
pub use life::LifeStateNet;
pub use menu::{MenuIdNet, MenuItemEnabledNet, MenuItemLabelNet, MenuItemView, MenuView};
pub use panel::{PanelButtonLabelNet, PanelButtonView, PanelNavOrderNet};
pub use selection::SelectionView;
pub use stat::{
    FactionNet, GangerNameNet, HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet,
};
pub use terrain::{
    DoorOpenNet, DoorView, EmplacementMannedNet, EmplacementView, GridHeightNet, GridLevelsNet,
    GridSizeNet, GridWidthNet, TerrainSummaryView,
};
pub use weapon::{FireModeLabel, FireModeView, WeaponNameNet, WeaponView};

#[cfg(test)]
mod test;
