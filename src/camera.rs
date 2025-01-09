use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Component)]
pub struct MainCamera;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum CameraSystem {
    UpdateCamera,
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                (
                    crate::anchor::bind_to_dyn_anchor,
                    crate::anchor::unbind_dyn_anchor,
                    camera_binded,
                    camera_move_to,
                ),
                crate::anchor::anchor,
            )
                .chain()
                .before(TransformSystem::TransformPropagate)
                .in_set(CameraSystem::UpdateCamera),
        );
    }
}

#[cfg(feature = "sequence")]
use bevy_sequence::prelude::*;

#[cfg(feature = "sequence")]
pub trait CameraCurveFragment<D, C>: Sized
where
    D: Threaded,
    C: Clone,
{
    /// Unbinds the camera and moves to the `marked` entity's position, with an offset, linearly over duration.
    fn move_camera_to<M: Component>(
        self,
        marker: M,
        offset: Vec2,
        duration: Duration,
    ) -> impl IntoFragment<D, C>
    where
        Self: IntoFragment<D, C>,
        D: Threaded;

    fn move_camera_curve<M: Component>(
        self,
        _marker: M,
        offset: Vec2,
        duration: Duration,
        curve: EaseFunction,
    ) -> impl IntoFragment<D, C>;

    /// Bind the camera to an entity's position.
    fn bind_camera<M: Component>(self, marker: M) -> impl IntoFragment<D, C>;

    /// Unbinds the camera and moves to the `marked` entity's position, with an offset, linearly
    /// over duration.
    ///
    /// After the move is complete, the camera binds to the `marked` entity.
    fn move_then_bind_camera<M: Component>(
        self,
        marker: M,
        offset: Vec2,
        duration: Duration,
    ) -> impl IntoFragment<D, C>;

    fn move_curve_then_bind_camera<M: Component>(
        self,
        _marker: M,
        offset: Vec2,
        duration: Duration,
        curve: EaseFunction,
    ) -> impl IntoFragment<D, C>;
}

#[cfg(feature = "sequence")]
impl<D, C, T> CameraCurveFragment<D, C> for T
where
    Self: IntoFragment<D, C>,
    D: Threaded,
    C: Threaded + Clone,
{
    fn move_camera_to<M: Component>(
        self,
        _marker: M,
        offset: Vec2,
        duration: Duration,
    ) -> impl IntoFragment<D, C> {
        let system = move |camera: Single<(Entity, &Transform), With<MainCamera>>,
                           entity_t: Single<&Transform, With<M>>,
                           mut commands: Commands| {
            let (camera, camera_t) = camera.into_inner();
            commands.entity(camera).insert(MoveTo::new(
                duration,
                camera_t.translation,
                entity_t.translation + offset.extend(0.),
                EaseFunction::Linear,
            ));
            commands.entity(camera).remove::<Binded>();
        };

        self.on_start(system)
    }

    fn move_camera_curve<M: Component>(
        self,
        _marker: M,
        offset: Vec2,
        duration: Duration,
        curve: EaseFunction,
    ) -> impl IntoFragment<D, C> {
        let system = move |camera: Single<(Entity, &Transform), With<MainCamera>>,
                           entity_t: Single<(&Transform, Option<&CameraOffset>), With<M>>,
                           mut commands: Commands| {
            let (entity_t, entity_offset) = entity_t.into_inner();
            let (camera, camera_t) = camera.into_inner();
            commands.entity(camera).insert(MoveTo::new(
                duration,
                camera_t.translation,
                entity_t.translation
                    + offset.extend(0.)
                    + entity_offset.map(|o| o.0).unwrap_or_default().extend(0.),
                curve,
            ));
            commands.entity(camera).remove::<Binded>();
        };

        self.on_start(system)
    }

    fn bind_camera<M: Component>(self, _marker: M) -> impl IntoFragment<D, C> {
        self.on_start(bind_camera::<M>)
    }

    fn move_then_bind_camera<M: Component>(
        self,
        _marker: M,
        offset: Vec2,
        duration: Duration,
    ) -> impl IntoFragment<D, C> {
        let mov = move |camera: Single<(Entity, &Transform), With<MainCamera>>,
                        entity_t: Single<&Transform, With<M>>,
                        mut commands: Commands| {
            let (camera, camera_t) = camera.into_inner();
            commands.entity(camera).insert(MoveTo::new(
                duration,
                camera_t.translation,
                entity_t.translation + offset.extend(0.),
                EaseFunction::Linear,
            ));
            commands.entity(camera).remove::<Binded>();
        };

        self.on_start(mov).on_end(bind_camera::<M>)
    }

    fn move_curve_then_bind_camera<M: Component>(
        self,
        _marker: M,
        offset: Vec2,
        duration: Duration,
        curve: EaseFunction,
    ) -> impl IntoFragment<D, C> {
        let system = move |camera: Single<(Entity, &Transform), With<MainCamera>>,
                           entity_t: Single<(&Transform, Option<&CameraOffset>), With<M>>,
                           mut commands: Commands| {
            let (entity_t, entity_offset) = entity_t.into_inner();
            let (camera, camera_t) = camera.into_inner();
            commands.entity(camera).insert(MoveTo::new(
                duration,
                camera_t.translation,
                entity_t.translation
                    + offset.extend(0.)
                    + entity_offset.map(|o| o.0).unwrap_or_default().extend(0.),
                curve,
            ));
        };

        self.on_start(system).on_end(bind_camera::<M>)
    }
}

#[derive(Component)]
#[component(on_insert = on_insert_moveto)]
pub struct MoveTo {
    timer: Timer,
    easing: EaseFunction,
    domain: Domain,
}

fn on_insert_moveto(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
    world.commands().entity(entity).remove::<Binded>();
}

impl MoveTo {
    pub fn new(duration: Duration, start: Vec3, end: Vec3, easing: EaseFunction) -> Self {
        Self {
            timer: Timer::new(duration, TimerMode::Once),
            easing,
            domain: Domain::Positions { start, end },
        }
    }

    pub fn new_with_entity(
        duration: Duration,
        start: Vec3,
        target: Entity,
        easing: EaseFunction,
    ) -> Self {
        Self {
            timer: Timer::new(duration, TimerMode::Once),
            easing,
            domain: Domain::Entity { start, end: target },
        }
    }

    pub fn tick(&mut self, duration: Duration) {
        self.timer.tick(duration);
    }

    pub fn complete(&self) -> bool {
        self.timer.finished()
    }
}

enum Domain {
    Entity { start: Vec3, end: Entity },
    Positions { start: Vec3, end: Vec3 },
}

impl Domain {
    pub fn target(&self) -> Option<Entity> {
        match self {
            Self::Entity { end, .. } => Some(*end),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct Binded(pub Entity);

#[derive(Debug, Default, Clone, Copy, Component)]
pub struct CameraOffset(pub Vec2);

pub fn bind_camera<M: Component>(
    entity: Option<Single<Entity, (With<M>, With<Transform>)>>,
    camera: Option<Single<Entity, With<MainCamera>>>,
    mut commands: Commands,
) {
    if let Some(camera) = camera {
        if let Some(entity) = entity {
            commands
                .entity(camera.into_inner())
                .insert(Binded(entity.into_inner()));
        } else {
            error_once!("Could not bind camera to entity: Entity not found");
        }
    } else {
        error_once!("Could not bind camera to entity: Camera not found");
    }
}

fn camera_move_to(
    camera: Option<Single<(Entity, &mut Transform, &mut MoveTo), With<MainCamera>>>,
    targets: Query<(&Transform, Option<&CameraOffset>), Without<MainCamera>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    if let Some((entity, mut transform, mut move_to)) = camera.map(|c| c.into_inner()) {
        move_to.tick(time.delta());

        if move_to.complete() {
            let mut entity = commands.entity(entity);
            entity.remove::<MoveTo>();

            if let Some(target) = move_to.domain.target() {
                entity.insert(Binded(target));
            }
        } else {
            let translation = match move_to.domain {
                Domain::Positions { start, end } => {
                    let curve = EasingCurve::new(start, end, move_to.easing);
                    curve.sample(move_to.timer.fraction())
                }
                Domain::Entity { start, end } => {
                    let Ok((target, offset)) = targets.get(end) else {
                        return;
                    };

                    let curve = EasingCurve::new(
                        start,
                        target.translation + offset.map(|o| o.0).unwrap_or_default().extend(0.),
                        move_to.easing,
                    );
                    curve.sample(move_to.timer.fraction())
                }
            };

            if let Some(t) = translation {
                transform.translation = t;
            }
        }
    }
}

fn camera_binded(
    camera: Option<Single<(&mut Transform, &Binded), With<MainCamera>>>,
    transforms: Query<(&Transform, Option<&CameraOffset>), Without<MainCamera>>,
) {
    if let Some((mut transform, binded)) = camera.map(|c| c.into_inner()) {
        if let Ok((t, offset)) = transforms.get(binded.0) {
            transform.translation =
                t.translation + offset.map(|o| o.0).unwrap_or_default().extend(0.);
        } else {
            warn_once!("Camera binded to entity with no transform");
        }
    }
}
