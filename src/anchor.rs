use crate::camera::{MainCamera, MoveTo};
use bevy::prelude::*;
use std::time::Duration;

/// Position which the [`MainCamera`] will snap to when a single instance exists.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component)]
#[require(Transform)]
pub struct CameraAnchor;

/// Position which the [`MainCamera`] will move to when an [`AnchorTarget`] enters the anchor's
/// translation and `radius`.
#[derive(Debug, Clone, Copy, PartialEq, Component)]
#[require(Transform)]
pub struct DynamicCameraAnchor {
    radius: f32,
    speed: f32,
}

impl DynamicCameraAnchor {
    pub fn new(radius: f32, speed: f32) -> Self {
        Self { radius, speed }
    }
}

/// Marks an entity as a valid target for triggering a [`DynamicCameraAnchor`] binding.
///
/// Only one can exist at any given time.
#[derive(Debug, Default, Clone, Copy, Component)]
#[require(Transform)]
pub struct AnchorTarget;

/// A component placed into the [`MainCamera`] which points to the entity currently dynamically
/// anchored to.
#[derive(Component)]
pub struct DynamicallyAnchored(Entity);

pub(crate) fn anchor(
    mut camera: Single<&mut Transform, With<MainCamera>>,
    anchor: Single<&Transform, (With<CameraAnchor>, Without<MainCamera>)>,
) {
    camera.translation = anchor.translation;
}

pub(crate) fn unbind_dyn_anchor(
    q: Query<(&DynamicCameraAnchor, &Transform)>,
    anchor_target: Single<(Entity, &Transform), With<AnchorTarget>>,
    camera: Single<(Entity, &Transform, &DynamicallyAnchored), With<MainCamera>>,
    mut commands: Commands,
) {
    let (camera, camera_transform, anchor_ref) = camera.into_inner();
    let (target, target_transform) = anchor_target.into_inner();
    let Ok((anchor, anchor_transform)) = q.get(anchor_ref.0) else {
        return;
    };

    if target_transform
        .translation
        .xy()
        .distance_squared(anchor_transform.translation.xy())
        .abs()
        > anchor.radius * anchor.radius
    {
        commands
            .entity(camera)
            .insert(MoveTo::new_with_entity(
                Duration::from_millis(anchor.speed as u64),
                camera_transform.translation,
                target,
                easing::EaseFunction::QuadraticOut,
            ))
            .remove::<DynamicallyAnchored>();
    }
}

pub(crate) fn bind_to_dyn_anchor(
    q: Query<(Entity, &DynamicCameraAnchor, &Transform)>,
    target_transform: Single<&Transform, With<AnchorTarget>>,
    camera: Single<(Entity, &Transform), (With<MainCamera>, Without<DynamicallyAnchored>)>,
    mut commands: Commands,
) {
    let (camera, camera_transform) = camera.into_inner();

    for (entity, anchor, transform) in q.iter() {
        if transform
            .translation
            .xy()
            .distance_squared(target_transform.translation.xy())
            .abs()
            <= anchor.radius * anchor.radius
        {
            commands.entity(camera).insert((
                MoveTo::new(
                    Duration::from_millis(anchor.speed as u64),
                    camera_transform.translation,
                    transform.translation,
                    easing::EaseFunction::QuadraticOut,
                ),
                DynamicallyAnchored(entity),
            ));
        }
    }
}
