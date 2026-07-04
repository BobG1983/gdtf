//! State-scoped RESOURCES for Bevy state machines (GTW-575).
//!
//! Bevy 0.19 scopes only ENTITIES to a state (`DespawnOnExit<S>` /
//! `DespawnOnEnter<S>`) — there is no built-in state-scoped *resource*
//! (bevy-traps #1). The house pattern is therefore an `OnEnter(state)` insert
//! plus an `OnExit(state)` remove, which every scene used to hand-stamp as a
//! pair of one-line systems. [`StateScopedResourceAppExt::init_state_scoped_resource`]
//! is that pair as ONE registration call, seeded by a caller-supplied
//! constructor.
//!
//! Consumer-side discipline is unchanged: any system reading a state-scoped
//! resource still guards with `.run_if(resource_exists::<R>)` or takes
//! `Option<Res<R>>` — this crate scopes the LIFETIME, it does not make the
//! resource safe to read unguarded.
//!
//! Shared by `gdtf_app`'s scene scaffolds and the content editor's
//! `Editing`-scoped model resources (the editor does not depend on `gdtf_app`,
//! so the seam lives in this tiny crate below both).

use bevy::{
    prelude::{App, Commands, OnEnter, OnExit, Resource},
    state::state::States,
};

/// One-call registration of a state-scoped resource: insert on `OnEnter`,
/// remove on `OnExit` (the bevy-traps #1 pattern, wired once instead of
/// hand-stamped per resource).
pub trait StateScopedResourceAppExt {
    /// Registers `OnEnter(state)` → `commands.insert_resource(seed())` and
    /// `OnExit(state)` → `commands.remove_resource::<R>()`.
    ///
    /// `seed` is a plain constructor FN (not a `Default` bound) because most
    /// scoped resources seed through a NAMED constructor carrying intent
    /// (`EditorMap::new`, `CurrentEditLevel::ground`, `CanvasZoom::identity`,
    /// …); `R::default` slots in wherever `Default` is the seed. The seed runs
    /// on EVERY entry, so a re-entered state starts from a fresh seed — exactly
    /// what the hand-stamped pairs did.
    ///
    /// The registered systems are ordinary [`Commands`]-param closures (never
    /// `&mut World` — bevy-traps #7), added UNORDERED within their transition
    /// schedules like the hand-stamped originals; order them at the call site
    /// if a scene ever needs it.
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

    /// A minimal two-state machine standing in for a scene lifecycle.
    #[derive(bevy::prelude::States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Phase {
        /// The state OUTSIDE the scoped span.
        #[default]
        Out,
        /// The state the resource is scoped to.
        In,
    }

    /// The scoped resource under test — a newtype so equality proves the SEED
    /// value arrived, not merely that some `Probe` exists.
    #[derive(Resource, Debug, PartialEq, Eq)]
    struct Probe(u8);

    impl Probe {
        /// The seed constructor the helper captures.
        const fn seeded() -> Self {
            Self(0xA5)
        }
    }

    /// Queues a `Phase` transition and applies it (`StateTransition` runs
    /// inside `update`).
    fn go(app: &mut App, phase: Phase) {
        app.world_mut()
            .resource_mut::<NextState<Phase>>()
            .set(phase);
        app.update();
        assert_eq!(*app.world().resource::<State<Phase>>().get(), phase);
    }

    /// Absent before enter, present-with-seed in-state, removed after exit —
    /// and re-seeded fresh on re-entry.
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

        // Re-entry seeds afresh — the scoped lifetime restarts per span.
        go(&mut app, Phase::In);
        assert_eq!(
            app.world().get_resource::<Probe>(),
            Some(&Probe::seeded()),
            "a re-entered state must re-seed the scoped resource",
        );
    }
}
