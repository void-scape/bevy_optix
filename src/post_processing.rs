use super::camera::MainCamera;
use bevy::{ecs::component::StorageType, prelude::*};
use std::marker::PhantomData;

/// Apply post processing to the main camera through an [`ApplyPostProcess`].
///
/// All [`Component`] types implement [`ApplyPostProcess`].
pub trait PostProcessCommand {
    /// Applies the post process to the [`MainCamera`].
    fn post_process(&mut self, post_process: impl ApplyPostProcess);

    /// Applies the post process to the [`MainCamera`], then binds the lifetime of the post process
    /// to the provided entity.
    fn bind_post_process(&mut self, post_process: impl ApplyPostProcess + Sync, entity: Entity);

    /// Removes the post process to the [`MainCamera`].
    fn remove_post_process<T: ApplyPostProcess>(&mut self);
}

impl PostProcessCommand for Commands<'_, '_> {
    fn post_process(&mut self, post_process: impl ApplyPostProcess) {
        self.queue(apply(post_process));
    }

    fn bind_post_process(&mut self, post_process: impl ApplyPostProcess + Sync, entity: Entity) {
        self.queue(bind(post_process, entity));
    }

    fn remove_post_process<T: ApplyPostProcess>(&mut self) {
        self.queue(remove::<T>);
    }
}

/// Determines how a post process is inserted and removed from the main camera.
pub trait ApplyPostProcess: 'static + Send {
    fn insert(self, entity: &mut EntityWorldMut<'_>);
    fn remove(entity: &mut EntityWorldMut<'_>);
}

impl<T: Component> ApplyPostProcess for T {
    fn insert(self, entity: &mut EntityWorldMut<'_>) {
        entity.insert(self);
    }

    fn remove(entity: &mut EntityWorldMut<'_>) {
        entity.remove::<T>();
    }
}

pub fn apply(post_process: impl ApplyPostProcess) -> impl FnOnce(&mut World) {
    move |world: &mut World| match world
        .query_filtered::<Entity, With<MainCamera>>()
        .get_single(world)
    {
        Ok(camera) => {
            post_process.insert(&mut world.entity_mut(camera));
        }
        Err(e) => {
            error!("failed to apply post process to main camera: {e}");
        }
    }
}

struct PostProcessBinding<T>(PhantomData<T>);

impl<T: ApplyPostProcess + Sync> Component for PostProcessBinding<T> {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_remove(|mut world, _, _| {
            world.commands().queue(remove::<T>);
        });
    }
}

pub fn bind<T: ApplyPostProcess + Sync>(
    post_process: T,
    entity: Entity,
) -> impl FnOnce(&mut World) {
    move |world: &mut World| match world
        .query_filtered::<Entity, With<MainCamera>>()
        .get_single(world)
    {
        Ok(camera) => {
            post_process.insert(&mut world.entity_mut(camera));
            world
                .entity_mut(entity)
                .with_child(PostProcessBinding::<T>(PhantomData));
        }
        Err(e) => {
            error!("failed to bind post process to main camera: {e}");
        }
    }
}

pub fn remove<T: ApplyPostProcess>(world: &mut World) {
    match world
        .query_filtered::<Entity, With<MainCamera>>()
        .get_single(world)
    {
        Ok(camera) => {
            T::remove(&mut world.entity_mut(camera));
        }
        Err(e) => {
            error!("failed to remove post process from main camera: {e}");
        }
    }
}
