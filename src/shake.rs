// MIT License
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Original from https://github.com/johanhelsing/bevy_trauma_shake
//!
//! Simple camera shake API with configurable [`ShakeSettings`] on a camera.

use bevy::prelude::*;

pub mod prelude {
    pub use super::{ScreenShakePlugin, Shake, ShakeSettings, TraumaCommands};
}

pub struct ScreenShakePlugin;

impl Plugin for ScreenShakePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_type::<Shake>()
            .register_type::<ShakeSettings>()
            .add_systems(PreUpdate, restore)
            .add_systems(
                PostUpdate,
                shake.before(TransformSystem::TransformPropagate),
            );
    }
}

#[derive(Component, Reflect, Clone, Debug)]
pub struct ShakeSettings {
    /// the amplitude of the shake, how far it can offset
    pub amplitude: f32,
    /// normally in the 2-3 range, a high power makes low traumas less intense
    pub trauma_power: f32,
    /// how much trauma is reduced each second
    pub decay_per_second: f32,
    /// how frequently noise can change from minimum to maximum
    pub frequency: f32,
    /// how many layers of noise (detail if you will)
    pub octaves: usize,
}

impl Default for ShakeSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ShakeSettings {
    const DEFAULT: ShakeSettings = ShakeSettings {
        trauma_power: 2.,
        decay_per_second: 0.8,
        amplitude: 100.,
        frequency: 15.,
        octaves: 1,
    };
}

/// Makes the entity shake according to applied trauma.
///
/// The shake happens during [`PostUpdate`], and the entity is restored to its
/// original translation in [`PreUpdate`]. This means that you can still control
/// the camera like you normally would inside update.
#[derive(Component, Reflect, Default, Clone, Debug)]
pub struct Shake {
    trauma: f32,
    reference_translation: Option<Vec3>,
}

impl Shake {
    /// Adds the specified trauma. Trauma is clamped between 0 and 1, and decays
    /// over time according to [`ShakeSettings::decay_per_second`].
    pub fn add_trauma(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).clamp(0., 1.);
    }
}

fn shake(mut shakes: Query<(&mut Shake, &mut Transform, Option<&ShakeSettings>)>, time: Res<Time>) {
    for (mut shake, mut transform, settings) in &mut shakes {
        let settings = settings.unwrap_or(&ShakeSettings::DEFAULT);

        let trauma = f32::max(
            shake.trauma - settings.decay_per_second * time.delta_secs(),
            0.0,
        );

        // avoid change detection
        if shake.trauma != trauma {
            shake.trauma = trauma;
        }

        let trauma_amount = f32::powf(shake.trauma, settings.trauma_power);

        if trauma_amount <= 0. {
            return;
        }

        shake.reference_translation = Some(transform.translation);

        let lacunarity = 2.;
        let gain = 0.5;
        let noise_pos = vec2(settings.frequency * time.elapsed_secs(), 0.);
        let offset = settings.amplitude
            * trauma_amount
            * Vec2::new(
                noise::fbm_simplex_2d(noise_pos + vec2(0., 1.), settings.octaves, lacunarity, gain),
                noise::fbm_simplex_2d(noise_pos + vec2(0., 2.), settings.octaves, lacunarity, gain),
            );

        transform.translation.x += offset.x;
        transform.translation.y += offset.y;
    }
}

fn restore(mut shakes: Query<(&mut Shake, &mut Transform)>) {
    for (mut shake, mut transform) in &mut shakes {
        // avoid change detection
        if shake.reference_translation.is_some() {
            let translation = shake.reference_translation.take().unwrap();
            transform.translation = translation;
        }
    }
}

/// Extension trait for [`Command`], adding commands for easily applying trauma
/// fire-and-forget-style.
pub trait TraumaCommands {
    /// Applies the given trauma to all `Shake`s
    /// ```
    /// # use bevy::prelude::*;
    /// use bevy_trauma_shake::prelude::*;
    ///
    /// fn add_shake(mut commands: Commands) {
    ///     commands.add_trauma(0.2);
    /// }
    /// ```
    fn add_trauma(&mut self, trauma: f32);
}

impl TraumaCommands for Commands<'_, '_> {
    fn add_trauma(&mut self, trauma: f32) {
        self.queue(AddTraumaCommand(trauma));
    }
}

struct AddTraumaCommand(f32);

impl Command for AddTraumaCommand {
    fn apply(self, world: &mut World) {
        for mut shake in world.query::<&mut Shake>().iter_mut(world) {
            shake.add_trauma(self.0);
        }
    }
}
