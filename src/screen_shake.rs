use bevy::prelude::*;
use rand::Rng;

pub struct ScreenShakePlugin;

impl Plugin for ScreenShakePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScreenShake::default())
            .add_systems(Update, screen_shake);
    }
}

#[derive(Default, Clone, Resource)]
pub struct ScreenShake {
    max_offset: f32,
    trauma: f32,
    camera_decay: f32,
    trauma_decay: f32,
}

impl ScreenShake {
    pub fn set(&mut self, max_offset: f32, camera_decay: f32, trauma_decay: f32) -> &mut Self {
        self.max_offset = max_offset;
        self.camera_decay = camera_decay;
        self.trauma_decay = trauma_decay;
        self
    }

    pub fn max_offset(&mut self, max_offset: f32) -> &mut Self {
        self.max_offset = max_offset;
        self
    }

    pub fn camera_decay(&mut self, camera_decay: f32) -> &mut Self {
        self.camera_decay = camera_decay;
        self
    }

    pub fn trauma_decay(&mut self, trauma_decay: f32) -> &mut Self {
        self.trauma_decay = trauma_decay;
        self
    }

    pub fn shake(&mut self) {
        self.shake_with(1.);
    }

    pub fn shake_with(&mut self, trauma: f32) {
        self.trauma = trauma;
    }
}

fn screen_shake(
    time: Res<Time>,
    mut screen_shake: ResMut<ScreenShake>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let mut rng = rand::thread_rng();
    let shake = screen_shake.trauma * screen_shake.trauma;
    let offset_x = screen_shake.max_offset * shake * rng.gen_range(-1.0..1.0);
    let offset_y = screen_shake.max_offset * shake * rng.gen_range(-1.0..1.0);

    if shake > 0.0 {
        for mut transform in query.iter_mut() {
            let target = transform.translation
                + Vec2 {
                    x: offset_x,
                    y: offset_y,
                }
                .extend(0.);
            transform.translation.smooth_nudge(
                &target,
                screen_shake.camera_decay,
                time.delta_secs(),
            );

            // let rotation = Quat::from_rotation_z(angle);
            // transform.rotation = transform
            //     .rotation
            //     .interpolate_stable(&(transform.rotation.mul_quat(rotation)), CAMERA_DECAY_RATE);
        }
    }

    screen_shake.trauma -= screen_shake.trauma_decay * time.delta_secs();
    screen_shake.trauma = screen_shake.trauma.clamp(0.0, 1.0);
}
