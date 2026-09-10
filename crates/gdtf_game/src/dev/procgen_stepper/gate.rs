use bevy::prelude::Resource;

crate::support_item! {
    /// Present while the stepper holds situation generation between stages.
    #[derive(Resource, Debug, Default, Clone, Copy)]
    struct ProcgenStepperActive;
}
