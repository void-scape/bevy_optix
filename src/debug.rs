use bevy::ecs::component::HookContext;
use bevy::ecs::system::RunSystemOnce;
use bevy::ecs::world::DeferredWorld;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Quick debug render with [`DebugRect`] and [`DebugCircle`].
///
/// Both spawn render components as children.
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugCircleAllocator::default());
    }
}

#[derive(Component)]
#[require(Transform, Visibility)]
#[component(on_add = Self::add_sprite)]
pub struct DebugRect {
    pub rect: Rect,
    pub color: Color,
}

impl DebugRect {
    pub fn new(rect: Rect, color: impl Into<Color>) -> Self {
        Self {
            rect,
            color: color.into(),
        }
    }

    pub fn from_size(size: Vec2) -> Self {
        Self::new(Rect::from_center_size(Vec2::ZERO, size), Color::WHITE)
    }

    pub fn from_size_color(size: Vec2, color: impl Into<Color>) -> Self {
        Self::new(Rect::from_center_size(Vec2::ZERO, size), color)
    }
}

#[derive(Component)]
#[require(Transform, Visibility)]
#[component(on_add = Self::add_mesh)]
pub struct DebugCircle {
    pub radius: f32,
    pub color: Color,
}

impl DebugCircle {
    pub fn new(radius: f32) -> Self {
        Self::color(radius, Color::WHITE)
    }

    pub fn color(radius: f32, color: impl Into<Color>) -> Self {
        Self {
            radius,
            color: color.into(),
        }
    }
}

#[derive(Default, Resource)]
struct DebugCircleAllocator {
    meshes: HashMap<u64, Handle<Mesh>>,
    materials: HashMap<[u8; 4], Handle<ColorMaterial>>,
}

impl DebugRect {
    fn add_sprite(mut world: DeferredWorld, ctx: HookContext) {
        world.commands().queue(move |world: &mut World| {
            world
                .run_system_once(
                    move |mut commands: Commands, debug_rects: Query<&DebugRect>| {
                        if let Ok(rect) = debug_rects.get(ctx.entity) {
                            commands.entity(ctx.entity).with_child(Sprite {
                                rect: Some(rect.rect),
                                color: rect.color,
                                ..Default::default()
                            });
                        }
                    },
                )
                .unwrap();
        });
    }
}

impl DebugCircle {
    fn add_mesh(mut world: DeferredWorld, ctx: HookContext) {
        world.commands().queue(move |world: &mut World| {
            world
                .run_system_once(
                    move |mut commands: Commands,
                          debug_circles: Query<&DebugCircle>,
                          mut allocator: ResMut<DebugCircleAllocator>,
                          mut meshes: ResMut<Assets<Mesh>>,
                          mut materials: ResMut<Assets<ColorMaterial>>| {
                        if let Ok(circle) = debug_circles.get(ctx.entity) {
                            let radius = (circle.radius * 1000.) as u64;

                            let mesh = allocator
                                .meshes
                                .entry(radius)
                                .or_insert_with(move || meshes.add(Circle::new(circle.radius)))
                                .clone();
                            let material = allocator
                                .materials
                                .entry(circle.color.to_srgba().to_u8_array())
                                .or_insert_with(move || {
                                    materials.add(ColorMaterial::from_color(circle.color))
                                })
                                .clone();

                            commands
                                .entity(ctx.entity)
                                .with_child((Mesh2d(mesh), MeshMaterial2d(material)));
                        }
                    },
                )
                .unwrap();
        });
    }
}
