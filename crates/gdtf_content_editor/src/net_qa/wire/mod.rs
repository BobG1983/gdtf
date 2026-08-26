//! Wire mirrors of the editor's own lifecycle state, mode tabs, and write outcomes.

mod armor;
mod attachment;
mod camera;
mod cell;
mod draft;
mod facing;
mod family;
mod field;
mod fire_mode;
mod grid;
mod injury;
mod injury_sub_tab;
mod key;
mod last_save;
mod list;
mod melee_weapon;
mod mode;
mod outcome;
mod painted;
mod pairing;
mod phase;
mod placement;
mod prefab_refusal;
mod refusal;
mod save_fault;
mod sprite;
mod stat_target;
mod terrain;
mod terrain_kind;
#[cfg(test)]
mod test;
mod tile_role;
mod toggle;
mod validation;
mod view;

pub(in crate::net_qa) use armor::{
    ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet, ArmorTypeNet,
    BodyPartNet,
};
pub(in crate::net_qa) use attachment::{
    AttachmentEffectNet, AttachmentSlotNet, DamageTypeNet, FatalBiasNet, WeaponDamageNet,
    WeaponPunchNet, WeaponShredNet,
};
pub(in crate::net_qa) use camera::{EditorPanNet, EditorZoomNet};
pub(in crate::net_qa) use cell::{EditorCellXNet, EditorCellYNet, EditorLevelNet};
pub(in crate::net_qa) use draft::{EditorDraftOutcomeNet, EditorDraftRonNet};
pub(in crate::net_qa) use facing::TerrainFacingNet;
pub(in crate::net_qa) use family::{
    EditorFamilyEntryNet, EditorFamilyLabelNet, EditorFamilyNet, EditorFamilyRowNet,
};
pub(in crate::net_qa) use field::{EditorDraftNameNet, EditorFieldNet};
pub(in crate::net_qa) use grid::{
    EditorGridHeightNet, EditorGridLevelsNet, EditorGridSizeNet, EditorGridWidthNet,
};
#[cfg(test)]
pub(in crate::net_qa) use injury::{BleedAmountNet, MovementCostFactorNet, StatDeltaNet};
pub(in crate::net_qa) use injury::{
    InjuryCategoryNet, InjuryEffectNet, InjuryKeyNet, InjurySeverityNet, InjuryTextNet,
};
pub(in crate::net_qa) use injury_sub_tab::InjurySubTabNet;
pub(in crate::net_qa) use key::{
    EditorContentNameNet, EditorKeyNet, SavedPathNet, TerrainKeyNet, ThemeKeyNet,
};
pub(in crate::net_qa) use last_save::{EditorLastSaveRowNet, LastSaveOutcomeNet};
pub(in crate::net_qa) use list::{
    EditorListIndexNet, EditorListMemberNet, EditorListNet, EditorListOpNet,
};
pub(in crate::net_qa) use melee_weapon::{
    AttachmentKeyNet, FightModeSpecNet, HandednessNet, ReachNet, ShoveNet, WeaponSlotNet,
};
#[cfg(test)]
pub(in crate::net_qa) use melee_weapon::{
    FightModeKindNet, SlotCapacityNet, StrikesNet, TuCostNet,
};
pub(in crate::net_qa) use mode::EditorModeNet;
pub(in crate::net_qa) use outcome::{
    EditorLoadOutcomeNet, EditorNewOutcomeNet, EditorSaveOutcomeNet,
};
pub(in crate::net_qa) use painted::{PaintedMapNet, PaintedRowNet};
pub(in crate::net_qa) use pairing::PairingOutcomeNet;
pub(in crate::net_qa) use phase::EditorPhaseNet;
pub(in crate::net_qa) use placement::PlacementVerdictNet;
pub(in crate::net_qa) use prefab_refusal::{PaintRefusalNet, SelectTileRefusalNet};
pub(in crate::net_qa) use refusal::EditorRefusalNet;
pub(in crate::net_qa) use save_fault::EditorSaveFaultNet;
pub(in crate::net_qa) use sprite::{
    SpriteAnimatedNet, SpriteFacingNet, SpriteFpsNet, SpritePxNet, SpriteSourceNet,
};
#[cfg(test)]
pub(in crate::net_qa) use stat_target::StatTargetNet;
pub(in crate::net_qa) use terrain::{
    BlocksPathingNet, FootfallNet, HeightBandNet, LosBlockingNet, MountedWeaponNet, TerrainHpNet,
    TerrainTagNet,
};
pub(in crate::net_qa) use terrain_kind::TerrainKindNet;
pub(in crate::net_qa) use tile_role::TileRoleNet;
pub(in crate::net_qa) use toggle::TerrainToggleNet;
pub(in crate::net_qa) use validation::{
    ChecksCompleteNet, ValidationFindingNet, ValidationPublishedNet,
};
pub(in crate::net_qa) use view::{EditorIsolateViewNet, EditorViewModeNet, EditorViewNet};
