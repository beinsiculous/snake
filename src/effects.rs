//! Visual effect presets — particle configs.
//!
//! Centralizes the look of each event (a snack eaten, a dog's death) so tuning
//! happens in one place. The colour is the caller's: the eaten snack's, or the dog's.

use engine_core::prelude::*;

/// Crumbs when a snack is eaten, in the snack's colour.
pub(crate) fn food_burst(theme: &ChaosTheme, tex: u32, color: Vec4) -> ParticleConfig {
    let count = (22.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.2, 0.5)
        .with_speed(90.0, 280.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(5.0, 0.5)
        .with_drag(2.4)
        .with_emissive(2.4)
        .with_texture(tex)
}

/// Big shatter at the head when a dog dies, in the dog's colour.
pub(crate) fn death_burst(theme: &ChaosTheme, tex: u32, color: Vec4) -> ParticleConfig {
    let count = (64.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.35, 0.9)
        .with_speed(120.0, 460.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(8.0, 0.5)
        .with_drag(1.8)
        .with_emissive(2.8)
        .with_texture(tex)
}
