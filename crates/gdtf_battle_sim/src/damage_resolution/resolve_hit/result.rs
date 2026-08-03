use bevy::prelude::Deref;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PenetratingDamage(i32);

impl PenetratingDamage {
        #[must_use]
    pub const fn new(pen: i32) -> Self {
        Self(pen)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HpDamage(i32);

impl HpDamage {
        #[must_use]
    pub const fn new(dmg: i32) -> Self {
        Self(dmg)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntegrityWear(i32);

impl IntegrityWear {
        #[must_use]
    pub const fn new(wear: i32) -> Self {
        Self(wear)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageMagnitude(i32);

impl DamageMagnitude {
        #[must_use]
    pub const fn new(magnitude: i32) -> Self {
        Self(magnitude)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct DamageReal(f32);

impl DamageReal {
        #[must_use]
    pub const fn new(real: f32) -> Self {
        Self(real)
    }

            #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
        pub penetrating: PenetratingDamage,
        pub hp_damage:   HpDamage,
        pub wear:        IntegrityWear,
}
