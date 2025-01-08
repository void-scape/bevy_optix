use bevy::prelude::*;

pub struct ZOrderPlugin;

impl Plugin for ZOrderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (order_z, origin_y).before(TransformSystem::TransformPropagate),
        );
    }
}

/// Determines the y offset from the entity's [`Transform`] by which the [`ZOrder`] is calculated.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct YOrigin(pub f32);

fn origin_y(
    mut commands: Commands,
    origin_query: Query<(Entity, &Transform, &YOrigin), Or<(Changed<Transform>, Changed<YOrigin>)>>,
) {
    for (entity, transform, origin) in origin_query.iter() {
        let order = -(origin.0 + transform.translation.y) / 10_000.;
        commands.entity(entity).insert(ZOrder(order));
    }
}

/// Describes the order that entities are drawn.
///
/// Use the [`YOrigin`] to generate a [`ZOrder`] automatically from the entities position.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct ZOrder(pub f32);

#[derive(Debug, Default, Clone, Copy, Component)]
struct UnorderedZ(pub f32);

fn order_z(
    mut commands: Commands,
    mut changed_order_query: Query<(&ZOrder, &UnorderedZ, &mut Transform), Changed<ZOrder>>,
    mut new_order_query: Query<
        (Entity, &ZOrder, &mut Transform),
        (Changed<ZOrder>, Without<UnorderedZ>),
    >,
) {
    for (entity, order, mut transform) in new_order_query.iter_mut() {
        commands
            .entity(entity)
            .insert(UnorderedZ(transform.translation.z));
        transform.translation.z += order.0;
    }

    for (order, unordered, mut transform) in changed_order_query.iter_mut() {
        transform.translation.z = unordered.0 + order.0;
    }
}
