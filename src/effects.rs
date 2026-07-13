//! Visual effect presets — particle configs.
//!
//! Centralizes the look of each event (food eaten, snake death) so tuning
//! happens in one place. The deforming grid uses the engine's
//! `default_playfield_grid` preset directly.

use engine_core::prelude::*;

use crate::constants::FOOD_COLOR;

/// Juicy pop when a pellet is eaten, tinted to the food color.
pub(crate) fn food_burst(theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (22.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.2, 0.5)
        .with_speed(90.0, 280.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(FOOD_COLOR, Vec4::new(FOOD_COLOR.x, FOOD_COLOR.y, FOOD_COLOR.z, 0.0))
        .with_scale(5.0, 0.5)
        .with_drag(2.4)
        .with_emissive(2.4)
        .with_texture(tex)
}

/// Big shatter at the head when the snake dies.
pub(crate) fn death_burst(theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let color = theme.accent_color;
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
