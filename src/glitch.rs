use crate::post_process::prelude::{PostProcessMaterial, PostProcessPlugin};
use bevy::asset::weak_handle;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderRef;
use bevy::{asset::load_internal_asset, prelude::*, render::render_resource::ShaderType};
use bevy_tween::{BevyTweenRegisterSystems, component_tween_system, prelude::Interpolator};

pub const GLITCH_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("b8f39834-a81e-4d5e-9ad9-043425f0afda");

pub struct GlitchPlugin;

impl Plugin for GlitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PostProcessPlugin::<GlitchSettings>::default())
            .add_tween_systems(component_tween_system::<TweenGlitch>())
            .add_systems(Update, tween_glitch);

        load_internal_asset!(
            app,
            GLITCH_SHADER_HANDLE,
            "shaders/glitch.wgsl",
            Shader::from_wgsl
        );
    }
}

#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct GlitchSettings {
    pub shake_power: f32,
    pub shake_rate: f32,
    pub shake_speed: f32,
    pub shake_block_size: f32,
    pub shake_color_rate: f32,
    pub intensity: f32,
}

impl Default for GlitchSettings {
    fn default() -> Self {
        Self {
            shake_power: 0.03,
            shake_rate: 0.5,
            shake_speed: 5.,
            shake_block_size: 30.5,
            shake_color_rate: 0.01,
            intensity: 0.5,
        }
    }
}

impl PostProcessMaterial for GlitchSettings {
    fn fragment_shader() -> ShaderRef {
        GLITCH_SHADER_HANDLE.into()
    }
}

impl GlitchSettings {
    pub fn from_intensity(intensity: f32) -> Self {
        Self {
            intensity,
            ..Default::default()
        }
    }
}

/// Describes the `intensity` of the screen's [`GlitchUniform`].
///
/// Use [`Single`] to access.
#[derive(Default, Component)]
pub struct GlitchIntensity(pub f32);

pub fn glitch_intensity(start: f32, end: f32) -> TweenGlitch {
    TweenGlitch::new(start, end)
}

#[derive(Component)]
pub struct TweenGlitch {
    start: f32,
    end: f32,
}

impl TweenGlitch {
    pub fn new(start: f32, end: f32) -> Self {
        Self { start, end }
    }
}

impl Interpolator for TweenGlitch {
    type Item = GlitchIntensity;

    fn interpolate(&self, item: &mut Self::Item, value: f32) {
        item.0 = self.start.lerp(self.end, value);
    }
}

fn tween_glitch(mut glitch_query: Query<(&mut GlitchSettings, &GlitchIntensity)>) {
    for (mut settings, intensity) in glitch_query.iter_mut() {
        settings.intensity = intensity.0;
    }
}
