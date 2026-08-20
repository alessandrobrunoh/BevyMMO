//! Lifecycle components for ability VFX entities.
//!
//! Each geometric effect spawned by an ability module carries **one** of these
//! components. The animator in `mod.rs` (or a dedicated system) ticks them
//! every frame and despawns the entity when the lifetime elapses.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Expand + fade-out (spheres, bursts)
// ---------------------------------------------------------------------------

/// Grows from `start_scale` to `end_scale` while fading, then despawns.
#[derive(Component)]
pub struct VfxExpandFade {
    pub elapsed: f32,
    pub duration: f32,
    pub start_scale: f32,
    pub end_scale: f32,
}

impl VfxExpandFade {
    pub fn new(duration: f32, start_scale: f32, end_scale: f32) -> Self {
        Self {
            elapsed: 0.0,
            duration,
            start_scale,
            end_scale,
        }
    }

    /// Returns `true` when the effect should be despawned.
    pub fn tick(&mut self, delta: f32, transform: &mut Transform) -> bool {
        self.elapsed += delta;
        if self.elapsed >= self.duration {
            return true;
        }
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        transform.scale = Vec3::splat(self.start_scale.lerp(self.end_scale, t));
        false
    }
}

// ---------------------------------------------------------------------------
// Pulse ring (ground circles with warning phase)
// ---------------------------------------------------------------------------

/// Pulses during a warning window, then expands on impact.
#[derive(Component)]
pub struct VfxPulseRing {
    pub elapsed: f32,
    pub warning_duration: f32,
    pub impact_duration: f32,
}

impl VfxPulseRing {
    pub fn new(warning_duration: f32, impact_duration: f32) -> Self {
        Self {
            elapsed: 0.0,
            warning_duration,
            impact_duration,
        }
    }

    pub fn tick(&mut self, delta: f32, transform: &mut Transform) -> bool {
        self.elapsed += delta;
        let total = self.warning_duration + self.impact_duration;
        if self.elapsed >= total {
            return true;
        }
        if self.elapsed < self.warning_duration {
            // Gentle pulse while waiting
            let pulse = 1.0 + (self.elapsed * 6.0).sin() * 0.06;
            transform.scale = Vec3::new(pulse, 1.0, pulse);
        } else {
            // Impact burst
            let t = ((self.elapsed - self.warning_duration) / self.impact_duration).clamp(0.0, 1.0);
            let burst = 1.0 + t * 0.6;
            transform.scale = Vec3::new(burst, 1.0, burst);
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Fixed-duration fade (materials that just need time-based alpha)
// ---------------------------------------------------------------------------

/// Stays at its initial transform for `duration` seconds, then despawns.
/// Used for shapes whose visual is static (e.g. a slash plane).
#[derive(Component)]
pub struct VfxLifetime {
    pub elapsed: f32,
    pub duration: f32,
}

impl VfxLifetime {
    pub fn new(duration: f32) -> Self {
        Self {
            elapsed: 0.0,
            duration,
        }
    }

    pub fn tick(&mut self, delta: f32) -> bool {
        self.elapsed += delta;
        self.elapsed >= self.duration
    }
}

// ---------------------------------------------------------------------------
// Falling / rising (rocks, orbs that move along Y)
// ---------------------------------------------------------------------------

/// Moves from start position to target Y over `fall_duration`, then despawns.
#[derive(Component)]
pub struct VfxFall {
    pub elapsed: f32,
    pub fall_duration: f32,
    pub target_y: f32,
    pub start_y: f32,
}

impl VfxFall {
    pub fn new(fall_duration: f32, start_y: f32, target_y: f32) -> Self {
        Self {
            elapsed: 0.0,
            fall_duration,
            target_y,
            start_y,
        }
    }

    pub fn tick(&mut self, delta: f32, transform: &mut Transform) -> bool {
        self.elapsed += delta;
        if self.elapsed >= self.fall_duration {
            return true;
        }
        let t = (self.elapsed / self.fall_duration).clamp(0.0, 1.0);
        transform.translation.y = self.start_y.lerp(self.target_y, t);
        false
    }
}

// ---------------------------------------------------------------------------
// Spin + expand (tornadoes, blade storms, swirling effects)
// ---------------------------------------------------------------------------

/// Rotates around Y while expanding, then despawns.
#[derive(Component)]
pub struct VfxSpinExpand {
    pub elapsed: f32,
    pub duration: f32,
    pub start_scale: f32,
    pub end_scale: f32,
    pub radians_per_sec: f32,
}

impl VfxSpinExpand {
    pub fn new(duration: f32, start_scale: f32, end_scale: f32, radians_per_sec: f32) -> Self {
        Self {
            elapsed: 0.0,
            duration,
            start_scale,
            end_scale,
            radians_per_sec,
        }
    }

    pub fn tick(&mut self, delta: f32, transform: &mut Transform) -> bool {
        self.elapsed += delta;
        if self.elapsed >= self.duration {
            return true;
        }
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        transform.scale = Vec3::splat(self.start_scale.lerp(self.end_scale, t));
        transform.rotate_y(self.radians_per_sec * delta);
        false
    }
}

// ---------------------------------------------------------------------------
// Oscillate (shields, fields that breathe)
// ---------------------------------------------------------------------------

/// Scales up and down in a sine wave for `duration` seconds.
#[derive(Component)]
pub struct VfxOscillate {
    pub elapsed: f32,
    pub duration: f32,
    pub base_scale: f32,
    pub amplitude: f32,
    pub frequency: f32, // Hz
}

impl VfxOscillate {
    pub fn new(duration: f32, base_scale: f32, amplitude: f32, frequency: f32) -> Self {
        Self {
            elapsed: 0.0,
            duration,
            base_scale,
            amplitude,
            frequency,
        }
    }

    pub fn tick(&mut self, delta: f32, transform: &mut Transform) -> bool {
        self.elapsed += delta;
        if self.elapsed >= self.duration {
            return true;
        }
        let wave = (self.elapsed * self.frequency * std::f32::consts::TAU).sin();
        transform.scale = Vec3::splat(self.base_scale + wave * self.amplitude);
        false
    }
}
