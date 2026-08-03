use std::time::Duration;

use bevy::prelude::*;

use super::sprite_map::GangerSprite;

const TWEEN_SECONDS: f32 = 0.2;

#[derive(Component, Debug, Clone)]
pub struct SpriteTween {
        source: Vec3,
        target: Vec3,
        clock:  Timer,
}

impl SpriteTween {
                                #[must_use]
    pub fn settled(position: Vec3) -> Self {
        let mut clock = Timer::from_seconds(TWEEN_SECONDS, TimerMode::Once);
        clock.tick(Duration::from_secs_f32(TWEEN_SECONDS));
        Self {
            source: position,
            target: position,
            clock,
        }
    }

                                        pub fn retarget(&mut self, from: Vec3, to: Vec3) {
        self.source = from;
        self.target = to;
        self.clock.reset();
    }

                                                    pub fn advance(&mut self, delta: Duration) -> Vec3 {
        if self.clock.tick(delta).is_finished() {
            return self.target;
        }
        let fraction = self.clock.fraction();
        self.source + (self.target - self.source) * fraction
    }
}

pub fn advance_sprite_tweens(
    time: Res<Time>,
    mut tweens: Query<(&mut Transform, &mut SpriteTween), With<GangerSprite>>,
) {
    let delta = time.delta();
    for (mut transform, mut tween) in &mut tweens {
        transform.translation = tween.advance(delta);
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use bevy::math::Vec3;

    use super::{SpriteTween, TWEEN_SECONDS};

            #[test]
    fn settled_tween_holds_position() {
        let at = Vec3::new(3.0, -4.0, 0.1);
        let mut tween = SpriteTween::settled(at);
        let held = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS));
        assert!(
            held.distance(at) < f32::EPSILON,
            "a settled tween must hold its position (got {held:?}, expected {at:?})",
        );
    }

                #[test]
    fn retargeted_tween_is_intermediate_then_settles() {
        let source = Vec3::new(0.0, 0.0, 0.1);
        let target = Vec3::new(10.0, 0.0, 0.1);
        let mut tween = SpriteTween::settled(source);
        tween.retarget(source, target);

        let mid = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS / 2.0));
        assert!(
            mid.x > source.x && mid.x < target.x,
            "a mid-glide tween must be strictly between source ({}) and target ({}), got {}",
            source.x,
            target.x,
            mid.x,
        );

        let done = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS));
        assert!(
            done.distance(target) < f32::EPSILON,
            "a finished tween must settle exactly on the target (got {done:?}, expected \
             {target:?})",
        );
    }

                    #[test]
    fn retarget_mid_glide_starts_from_current_position() {
        let a = Vec3::new(0.0, 0.0, 0.1);
        let b = Vec3::new(10.0, 0.0, 0.1);
        let mut tween = SpriteTween::settled(a);
        tween.retarget(a, b);

        let mid = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS / 2.0));
        assert!(mid.x > a.x && mid.x < b.x, "partway toward b");

        let c = Vec3::new(mid.x, 10.0, 0.1);
        tween.retarget(mid, c);
        let after = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS / 4.0));
        assert!(
            after.y > mid.y && after.y < c.y,
            "after a mid-glide retarget the glide must continue from the current position \
             ({mid:?}) toward the new target ({c:?}), got {after:?}",
        );
        assert!(
            (after.x - mid.x).abs() < 1.0e-4,
            "the x stays at the mid point (no snap-back to a's x), got {} vs mid {}",
            after.x,
            mid.x,
        );
    }
}
