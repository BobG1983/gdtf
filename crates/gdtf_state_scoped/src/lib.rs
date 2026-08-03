use bevy::{
    prelude::{App, Commands, OnEnter, OnExit, Resource},
    state::state::States,
};

pub trait StateScopedResourceAppExt {
                                                            fn init_state_scoped_resource<S: States, R: Resource>(
        &mut self,
        state: S,
        seed: impl Fn() -> R + Send + Sync + 'static,
    ) -> &mut Self;
}

impl StateScopedResourceAppExt for App {
    fn init_state_scoped_resource<S: States, R: Resource>(
        &mut self,
        state: S,
        seed: impl Fn() -> R + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_systems(OnEnter(state.clone()), move |mut commands: Commands| {
            commands.insert_resource(seed());
        })
        .add_systems(OnExit(state), |mut commands: Commands| {
            commands.remove_resource::<R>();
        })
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        prelude::{App, AppExtStates as _, NextState, Resource, State},
        state::app::StatesPlugin,
    };

    use super::StateScopedResourceAppExt;

        #[derive(bevy::prelude::States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Phase {
                #[default]
        Out,
                In,
    }

            #[derive(Resource, Debug, PartialEq, Eq)]
    struct Probe(u8);

    impl Probe {
                const fn seeded() -> Self {
            Self(0xA5)
        }
    }

            fn go(app: &mut App, phase: Phase) {
        app.world_mut()
            .resource_mut::<NextState<Phase>>()
            .set(phase);
        app.update();
        assert_eq!(*app.world().resource::<State<Phase>>().get(), phase);
    }

            #[test]
    fn insert_on_enter_remove_on_exit() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<Phase>();
        app.init_state_scoped_resource(Phase::In, Probe::seeded);
        app.update();

        assert!(
            app.world().get_resource::<Probe>().is_none(),
            "the scoped resource must be ABSENT before its state is entered",
        );

        go(&mut app, Phase::In);
        assert_eq!(
            app.world().get_resource::<Probe>(),
            Some(&Probe::seeded()),
            "OnEnter must insert the resource with the caller-supplied seed value",
        );

        go(&mut app, Phase::Out);
        assert!(
            app.world().get_resource::<Probe>().is_none(),
            "OnExit must remove the scoped resource",
        );

        go(&mut app, Phase::In);
        assert_eq!(
            app.world().get_resource::<Probe>(),
            Some(&Probe::seeded()),
            "a re-entered state must re-seed the scoped resource",
        );
    }
}
