use std::marker::PhantomData;

use bevy::ecs::component::HookContext;
use bevy::ecs::system::RunSystemOnce;
use bevy::ecs::world::DeferredWorld;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::pixel_perfect::HIGH_RES_LAYER;

/// Quick debug render primitives.
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugCircleAllocator::default());
    }
}

pub trait DebugComponentAppExt {
    fn debug_component<T: Component + core::fmt::Debug>(&mut self) -> &mut Self;
}

impl DebugComponentAppExt for App {
    fn debug_component<T: Component + core::fmt::Debug>(&mut self) -> &mut Self {
        self.add_systems(Update, (init_debug_component::<T>, debug_component::<T>))
    }
}

pub fn debug_res<R: Resource + core::fmt::Debug>(
    transform: Transform,
    anchor: bevy::sprite::Anchor,
) -> impl Fn(Commands, Local<Option<Entity>>, Res<R>) {
    move |mut commands, mut text, res| {
        if res.is_changed() {
            let entity = text.get_or_insert_with(|| {
                commands
                    .spawn((Text2d::default(), HIGH_RES_LAYER, transform, anchor))
                    .id()
            });

            let mut entity = match commands.get_entity(*entity) {
                Ok(entity) => entity,
                Err(_) => commands.spawn((Text2d::default(), HIGH_RES_LAYER, transform, anchor)),
            };

            entity.insert(Text2d::new(format!("{:?}", res.as_ref())));
        }
    }
}

pub fn debug_single<C: Component + core::fmt::Debug>(
    transform: Transform,
    anchor: bevy::sprite::Anchor,
) -> impl Fn(Commands, Local<Option<Entity>>, Single<Ref<C>>) {
    move |mut commands, mut text, single| {
        if single.is_changed() {
            let entity = text.get_or_insert_with(|| {
                commands
                    .spawn((Text2d::default(), HIGH_RES_LAYER, transform, anchor))
                    .id()
            });

            let mut entity = match commands.get_entity(*entity) {
                Ok(entity) => entity,
                Err(_) => commands.spawn((Text2d::default(), HIGH_RES_LAYER, transform, anchor)),
            };

            entity.insert(Text2d::new(format!("{:?}", single.into_inner().as_ref())));
        }
    }
}

#[derive(Component)]
pub struct DebugComponent<T>(fn(&mut EntityCommands), PhantomData<fn(T)>);

impl<T> DebugComponent<T> {
    pub fn new(bundle: fn(&mut EntityCommands)) -> Self {
        Self(bundle, PhantomData)
    }
}

#[derive(Component)]
struct Debugged;

fn init_debug_component<T: Component>(
    mut commands: Commands,
    debug: Query<(Entity, &DebugComponent<T>), (Without<Debugged>, With<T>)>,
) {
    for (entity, debug) in debug.iter() {
        commands.entity(entity).insert(Debugged);
        let mut child = commands.spawn(Text2d::default());
        (debug.0)(&mut child);
        child.insert(ChildOf(entity));
    }
}

fn debug_component<T: Component + core::fmt::Debug>(
    debug: Query<(&Children, Ref<T>), With<DebugComponent<T>>>,
    mut text: Query<&mut Text2d>,
) {
    for (children, val) in debug.iter() {
        if val.is_changed() {
            let mut iter = text.iter_many_mut(children.iter());
            while let Some(mut text) = iter.fetch_next() {
                text.0 = format!("{:?}", val.as_ref());
            }
        }
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

#[derive(Clone, Component)]
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
                            commands.entity(ctx.entity).insert(Sprite {
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
                                .insert((Mesh2d(mesh), MeshMaterial2d(material)));
                        }
                    },
                )
                .unwrap();
        });
    }
}
