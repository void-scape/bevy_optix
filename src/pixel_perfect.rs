use super::camera::MainCamera;
use bevy::prelude::*;
use bevy::{
    image::ImageSamplerDescriptor,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
    window::WindowResized,
};

pub const HIGH_RES_LAYER: RenderLayers = RenderLayers::layer(1);

/// Determines the resolution of the [`MainCamera`].
#[derive(Debug, Clone, Copy, Resource)]
pub struct CanvasDimensions {
    pub width: u32,
    pub height: u32,
    pub pixel_scale: f32,
}

/// Captures the `pixel_perfect::HIGH_RES_BACKGROUND_LAYER` and `pixel_perfect::HIGH_RES_LAYER`, rendering the [`Canvas`] texture generated from the
/// [`MainCamera`] inbetween these two high resolution layers.
#[derive(Component)]
pub struct OuterCamera;

/// If this resource exists, then move the [`Canvas`] and [`OuterCamera`] to the position of the [`MainCamera`].
///
/// This enables the outer camera to capture anything positioned within the `pixel_perfect::HIGH_RES_BACKGROUND_LAYER` and
/// `pixel_perfect::HIGH_RES_LAYER` render layers.
#[derive(Debug, Resource)]
pub struct AlignCanvasToCamera;

/// Determines what will be scaled in order for the canvas to fill the screen.
#[derive(Debug, Resource)]
pub enum Scaling {
    /// Scales the mesh canvas.
    ///
    /// Violates position and size cohesion between high res and low res layers.
    /// Prevents resolution scaling of high res layers.
    Canvas,
    /// Scales the camera projection.
    ///
    /// Retains position and size cohesion between res layers.
    /// Results in wacky scaling on the high res layer as window size changes.
    Projection,
}

pub struct PixelPerfectPlugin(pub CanvasDimensions);

impl Plugin for PixelPerfectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0)
            //.insert_resource(AlignCanvasToCamera)
            //.insert_resource(Scaling::Projection)
            .add_systems(PreStartup, setup_cameras)
            .add_systems(First, (fit_canvas, resize_canvas, propagate_render_layers));
        //.add_systems(
        //    PostUpdate,
        //    align_canvas_to_camera
        //        .before(TransformSystem::TransformPropagate)
        //        .after(CameraSystem::UpdateCamera)
        //        .run_if(resource_exists::<AlignCanvasToCamera>),
        //);

        // .add_systems(
        //     PostUpdate,
        //     (correct_camera
        //         .after(CameraSystem::UpdateCamera)
        //         .before(TransformSystem::TransformPropagate),),
        // )
        // .add_systems(PreUpdate, remove_offset);
    }
}

#[derive(Component)]
pub struct Canvas;

fn setup_cameras(mut commands: Commands, dimensions: Res<CanvasDimensions>) {
    commands.spawn((
        Canvas,
        Transform::from_xyz(0., 0., -999.9).with_scale(Vec3::splat(dimensions.pixel_scale)),
        HIGH_RES_LAYER,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            hdr: true,
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..Default::default()
        },
        MainCamera,
        Msaa::Off,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            hdr: true,
            order: 1,
            ..Default::default()
        },
        OuterCamera,
        HIGH_RES_LAYER,
        Msaa::Off,
    ));
}

fn fit_canvas(
    dimensions: Res<CanvasDimensions>,
    mut resize_events: EventReader<WindowResized>,
    mut projection: Single<&mut Projection, With<OuterCamera>>,
) {
    for event in resize_events.read() {
        let h_scale = event.width / dimensions.width as f32;
        let v_scale = event.height / dimensions.height as f32;
        let scale = h_scale.min(v_scale) / dimensions.pixel_scale;

        if let Projection::Orthographic(projection) = projection.as_mut() {
            projection.scale = 1. / scale;
        }
    }
}

fn resize_canvas(
    mut commands: Commands,
    dimensions: Res<CanvasDimensions>,
    mut images: ResMut<Assets<Image>>,
    mut camera: Single<&mut Camera, With<MainCamera>>,
    canvas: Single<Entity, With<Canvas>>,
) {
    if !dimensions.is_changed() {
        return;
    }

    let canvas_size = Extent3d {
        width: dimensions.width,
        height: dimensions.height,
        ..default()
    };

    info!("resizing pixel perfect canvas: {:?}", canvas_size);
    let mut new_canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: canvas_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::bevy_default(),
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        sampler: bevy::image::ImageSampler::Descriptor(ImageSamplerDescriptor::nearest()),
        ..default()
    };

    new_canvas.resize(canvas_size);
    let handle = images.add(new_canvas);
    camera.target = RenderTarget::Image(handle.clone().into());
    commands.entity(*canvas).insert(Sprite::from_image(handle));
}

fn propagate_render_layers(
    mut commands: Commands,
    parents: Query<(&Children, &RenderLayers), Or<(Changed<RenderLayers>, Changed<Children>)>>,
) {
    for (children, layers) in parents.iter() {
        for child in children.iter() {
            commands.entity(child).insert(layers.clone());
        }
    }
}

//fn align_canvas_to_camera(
//    mut cameras: Query<&mut Transform, (With<OuterCamera>, Without<Canvas>)>,
//    canvas: Single<&mut Transform, (With<Canvas>, Without<OuterCamera>)>,
//    main_camera: Single<&Transform, (With<MainCamera>, Without<OuterCamera>, Without<Canvas>)>,
//) {
//    for mut camera in cameras.iter_mut() {
//        camera.translation = main_camera.translation;
//    }
//    canvas.into_inner().translation = main_camera.translation;
//}

// #[derive(Component)]
// struct TempOffset(Vec3);
//
// fn correct_camera(
//     mut commands: Commands,
//     main_camera_query: Option<Single<(&mut Transform, Option<&Binded>), With<MainCamera>>>,
//     outer_camera_query: Option<Single<&mut Transform, (With<OuterCamera>, Without<MainCamera>)>>,
//     mut binded_query: Query<&mut Transform, (Without<MainCamera>, Without<OuterCamera>)>,
// ) {
//     if let Some((mut inner, binded)) = main_camera_query.map(|q| q.into_inner()) {
//         if let Some(mut outer) = outer_camera_query.map(|q| q.into_inner()) {
//             let rounded = inner.translation.round();
//             outer.translation = inner.translation - rounded;
//             inner.translation = rounded;
//
//             if let Some((entity, Ok(mut binded))) = binded.map(|b| (b.0, binded_query.get_mut(b.0)))
//             {
//                 let offset = binded.translation - rounded;
//                 binded.translation -= offset;
//                 commands.entity(entity).insert(TempOffset(offset));
//             }
//         }
//     }
// }
//
// fn remove_offset(
//     mut commands: Commands,
//     mut offset_query: Query<(Entity, &mut Transform, &TempOffset)>,
// ) {
//     for (entity, mut transform, offset) in offset_query.iter_mut() {
//         transform.translation += offset.0;
//         commands.entity(entity).remove::<TempOffset>();
//     }
// }
