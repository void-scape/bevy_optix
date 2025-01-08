use crate::camera::{MainCamera, MoveTo};
use bevy::prelude::*;
use std::time::Duration;

/// Position which the [`MainCamera`] will snap to when a single instance exists.
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct CameraAnchor;

/// Position which the [`MainCamera`] will move to when an [`AnchorTarget`] enters the anchor's
/// translation and `radius`.
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct DynamicCameraAnchor {
    radius: f32,
}

/// Marks an entity as a valid target for triggering a [`DynamicCameraAnchor`] binding.
///
/// Only one can exist at any given time.
#[derive(Debug, Clone, Copy, Component)]
pub struct AnchorTarget;

/// A component placed into the [`MainCamera`] which points to the entity currently dynamically
/// anchored to.
#[derive(Component)]
pub struct DynamicallyAnchored(Entity);

pub(crate) fn anchor(
    camera: Option<Single<&mut Transform, With<MainCamera>>>,
    anchor: Query<&Transform, (With<CameraAnchor>, Without<MainCamera>)>,
) {
    match anchor.get_single() {
        Ok(t) => {
            if let Some(mut camera) = camera {
                camera.translation = t.translation;
            }
        }
        Err(e) => warn_once!("could not anchor camera: {e}"),
    }
}

pub(crate) fn unbind_dyn_anchor(
    q: Query<(&DynamicCameraAnchor, &Transform)>,
    anchor_target: Query<(Entity, &Transform), With<AnchorTarget>>,
    camera: Query<(Entity, &Transform, &DynamicallyAnchored), With<MainCamera>>,
    mut commands: Commands,
) {
    let Ok((camera, camera_transform, anchor_ref)) = camera.get_single() else {
        return;
    };

    let Ok((anchor, anchor_transform)) = q.get(anchor_ref.0) else {
        return;
    };

    let Ok((target, target_transform)) = anchor_target.get_single() else {
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
    target: Query<&Transform, With<AnchorTarget>>,
    camera: Query<(Entity, &Transform), (With<MainCamera>, Without<DynamicallyAnchored>)>,
    mut commands: Commands,
) {
    let Ok((camera, camera_transform)) = camera.get_single() else {
        return;
    };

    let Ok(target_transform) = target.get_single() else {
        return;
    };

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
