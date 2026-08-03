//! authored field round-trips by construction).

use bevy::prelude::*;
use gdtf_battle_sim::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection, ArmorSpec,
    ArmorType, BodyPart,
};

const SEED_PIECE: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
        Pending,
        Done,
}

#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct ArmorDraft {
        name:     String,
        spec:     ArmorSpec,
        autoload: AutoloadState,
}

impl ArmorDraft {
                    #[must_use]
    pub const fn new_armor() -> Self {
        Self {
            name:     String::new(),
            spec:     ArmorSpec::uniform(SEED_PIECE),
            autoload: AutoloadState::Done,
        }
    }

                #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

            pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

                pub fn load_armor(&mut self, name: &ArmorName, spec: &ArmorSpec) {
        name.as_str().clone_into(&mut self.name);
        self.spec = *spec;
        self.autoload = AutoloadState::Done;
    }

        #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

        pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

        #[must_use]
    pub const fn spec(&self) -> &ArmorSpec {
        &self.spec
    }

                        #[must_use]
    pub const fn piece_mut(&mut self, part: BodyPart) -> &mut ArmorPiece {
        match part {
            BodyPart::Head => &mut self.spec.head,
            BodyPart::Torso => &mut self.spec.torso,
            BodyPart::LeftArm => &mut self.spec.left_arm,
            BodyPart::RightArm => &mut self.spec.right_arm,
            BodyPart::LeftLeg => &mut self.spec.left_leg,
            BodyPart::RightLeg => &mut self.spec.right_leg,
        }
    }
}

impl Default for ArmorDraft {
                    fn default() -> Self {
        Self {
            name:     String::new(),
            spec:     ArmorSpec::uniform(SEED_PIECE),
            autoload: AutoloadState::Pending,
        }
    }
}
