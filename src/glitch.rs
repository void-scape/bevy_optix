use crate::pixel_perfect::Canvas;
use bevy::{
    asset::load_internal_asset,
    ecs::query::QuerySingleError,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    sprite::{Material2d, Material2dPlugin},
};
use bevy_tween::{component_tween_system, prelude::Interpolator, BevyTweenRegisterSystems};

pub const GLITCH_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0x19A72E656);

pub struct GlitchPlugin;

impl Plugin for GlitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<Glitch>::default())
            .add_tween_systems(component_tween_system::<TweenGlitch>())
            .init_asset::<Glitch>()
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn(GlitchIntensity::default());
            })
            .add_systems(Update, tween_glitch);

        load_internal_asset!(
            app,
            GLITCH_SHADER_HANDLE,
            "shaders/glitch.wgsl",
            Shader::from_wgsl
        );
    }
}

/// Set the post processing glitch `INTENSITY` from 0-10.
pub fn set_glitch_intensity<const INTENSITY: usize>(
    canvas: Option<Single<&MeshMaterial2d<Glitch>, With<Canvas>>>,
    mut glitches: ResMut<Assets<Glitch>>,
) {
    if let Some(handle) = canvas {
        let Some(glitch) = glitches.get_mut(*handle) else {
            error!("failed to set glitch intensity: `Glitch` not found in `Assets<Glitch>`");
            return;
        };

        glitch.uniform.intensity = INTENSITY as f32 / 10.;
    } else {
        warn!("failed to set glitch intensity: `Canvas` is not found (spawned in `PixelPerfectPlugin`)");
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

fn tween_glitch(
    glitch_query: Query<&GlitchIntensity>,
    canvas: Option<Single<&MeshMaterial2d<Glitch>, With<Canvas>>>,
    mut glitches: ResMut<Assets<Glitch>>,
) {
    match glitch_query.get_single() {
        Ok(intensity) => {
            if let Some(handle) = canvas {
                let Some(glitch) = glitches.get_mut(*handle) else {
                    error!(
                        "failed to set glitch intensity: `Glitch` not found in `Assets<Glitch>`"
                    );
                    return;
                };

                if glitch.uniform.intensity != intensity.0 {
                    glitch.uniform.intensity = intensity.0;
                }
            } else {
                warn!("failed to set glitch intensity: `Canvas` is not found (spawned in `PixelPerfectPlugin`)");
            }
        }
        Err(QuerySingleError::MultipleEntities(err)) => {
            error_once!("(warns once) failed to tween the screen `Glitch`: {err}");
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Asset, TypePath, AsBindGroup)]
pub struct Glitch {
    #[texture(0)]
    #[sampler(1)]
    pub image: Handle<Image>,
    #[uniform(2)]
    pub uniform: GlitchUniform,
}

impl Glitch {
    pub fn new(image: Handle<Image>, uniform: GlitchUniform) -> Self {
        Self { image, uniform }
    }

    pub fn from_image(image: Handle<Image>) -> Self {
        Self {
            image,
            uniform: GlitchUniform::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, AsBindGroup, TypePath, ShaderType)]
#[uniform(2, GlitchUniform)]
pub struct GlitchUniform {
    pub shake_power: f32,
    pub shake_rate: f32,
    pub shake_speed: f32,
    pub shake_block_size: f32,
    pub shake_color_rate: f32,
    pub intensity: f32,
}

impl GlitchUniform {
    pub fn from_intensity(intensity: f32) -> Self {
        Self {
            intensity,
            ..Default::default()
        }
    }
}

impl From<&GlitchUniform> for GlitchUniform {
    fn from(value: &GlitchUniform) -> Self {
        *value
    }
}

impl Default for GlitchUniform {
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

impl Material2d for Glitch {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        GLITCH_SHADER_HANDLE.into()
    }
}
