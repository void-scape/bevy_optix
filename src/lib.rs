use bevy::prelude::*;

pub mod anchor;
pub mod camera;
pub mod pixel_perfect;
pub mod post_processing;
pub mod zorder;

pub struct PixelGfx;

impl Plugin for PixelGfx {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            zorder::ZOrderPlugin,
            pixel_perfect::PixelPerfectPlugin,
        ));
    }
}
