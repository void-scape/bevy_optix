use super::camera::{CameraSystem, MainCamera};
use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    image::ImageSamplerDescriptor,
    prelude::*,
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
#[derive(Debug, Clone, Copy, Component)]
pub struct CanvasDimensions {
    pub width: u32,
    pub height: u32,
}

/// Captures the [`HIGH_RES_LAYER`], including the [`Canvas`] texture generated from the
/// [`MainCamera`].
#[derive(Component)]
pub struct OuterCamera;

/// If true, moves the [`Canvas`] and [`OuterCamera`] to the position of the [`MainCamera`].
///
/// This enables the out camera to capture anything positioned within the [`HIGH_RES_LAYER`] render
/// layer.
#[derive(Debug, Resource)]
pub struct AlignCanvasToCamera(pub bool);

pub struct PixelPerfectPlugin(pub CanvasDimensions);

impl Plugin for PixelPerfectPlugin {
    fn build(&self, app: &mut App) {
        setup_camera(app.world_mut(), self.0);
        app.insert_resource(self.0)
            .insert_resource(AlignCanvasToCamera(true))
            .add_systems(Update, fit_canvas)
            .add_systems(
                PostUpdate,
                align_canvas_to_camera
                    .before(TransformSystem::TransformPropagate)
                    .after(CameraSystem::UpdateCamera),
            );
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
struct Canvas;

fn setup_camera(world: &mut World, dimensions: CanvasDimensions) {
    let canvas_size = Extent3d {
        width: dimensions.width,
        height: dimensions.height,
        ..default()
    };

    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: canvas_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
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

    canvas.resize(canvas_size);
    let image_handle = world.add_asset(canvas);

    world.commands().spawn((
        Camera2d,
        Camera {
            hdr: true,
            order: -1,
            target: RenderTarget::Image(image_handle.clone()),
            ..Default::default()
        },
        Tonemapping::TonyMcMapface,
        MainCamera,
        Msaa::Off,
        dimensions,
    ));

    world
        .commands()
        .spawn((Sprite::from_image(image_handle), Canvas, HIGH_RES_LAYER));

    world.commands().spawn((
        Camera2d,
        Camera {
            hdr: true,
            ..Default::default()
        },
        OuterCamera,
        HIGH_RES_LAYER,
        Msaa::Off,
    ));
}

fn fit_canvas(
    dimensions: Single<&CanvasDimensions, With<MainCamera>>,
    mut resize_events: EventReader<WindowResized>,
    mut projection: Single<&mut OrthographicProjection, With<OuterCamera>>,
) {
    for event in resize_events.read() {
        let h_scale = event.width / dimensions.width as f32;
        let v_scale = event.height / dimensions.height as f32;
        projection.scale = 1. / h_scale.min(v_scale);
    }
}

fn resize_canvas(
    dimensions: Single<&CanvasDimensions, (With<MainCamera>, Changed<CanvasDimensions>)>,
    mut canvas: Single<&mut Sprite, With<Canvas>>,
    window: Single<&Window>,
    mut projection: Single<&mut OrthographicProjection, With<OuterCamera>>,
    mut images: ResMut<Assets<Image>>,
) {
    let canvas_size = Extent3d {
        width: dimensions.width,
        height: dimensions.height,
        ..default()
    };

    let mut new_canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: canvas_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
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
    canvas.image = images.add(new_canvas);

    let h_scale = window.resolution.width() / dimensions.width as f32;
    let v_scale = window.resolution.height() / dimensions.height as f32;
    projection.scale = 1. / h_scale.min(v_scale);
}

fn align_canvas_to_camera(
    align: Res<AlignCanvasToCamera>,
    camera: Single<&mut Transform, (With<OuterCamera>, Without<Canvas>)>,
    canvas: Single<&mut Transform, (With<Canvas>, Without<OuterCamera>)>,
    main_camera: Single<&Transform, (With<MainCamera>, Without<OuterCamera>, Without<Canvas>)>,
) {
    if align.0 {
        camera.into_inner().translation = main_camera.translation;
        canvas.into_inner().translation = main_camera.translation;
    } else {
        camera.into_inner().translation = Vec3::ZERO;
        canvas.into_inner().translation = Vec3::ZERO;
    }
}

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
