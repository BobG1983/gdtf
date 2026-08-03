use bevy::prelude::{Res, Resource};

crate::support_item! {
                                                                    #[derive(Resource, Debug, Default, Clone, Copy)]
    struct ProcgenStepperActive;
}

/// battle instead — see `battle_sim::plugin`'s `cfg(feature = "dev_tools")` wiring for the
#[must_use]
pub(crate) const fn battle_setup_runs_directly(active: Option<Res<ProcgenStepperActive>>) -> bool {
    active.is_none()
}
