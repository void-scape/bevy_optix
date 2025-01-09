#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use pixel_perfect::CanvasDimensions;

pub mod anchor;
pub mod camera;
pub mod pixel_perfect;
pub mod post_processing;
pub mod zorder;

pub struct PixelGfxPlugin(pub CanvasDimensions);

impl Plugin for PixelGfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            zorder::ZOrderPlugin,
            pixel_perfect::PixelPerfectPlugin(self.0),
        ));
    }
}
