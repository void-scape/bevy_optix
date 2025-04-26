use bevy::{
    ecs::component::{Mutable, StorageType},
    prelude::*,
};
use std::marker::PhantomData;

/// Apply post processing to the main camera through an [`ApplyPostProcess`].
///
/// All [`Component`] types implement [`ApplyPostProcess`].
pub trait PostProcessCommand {
    /// Applies the post process to the camera with `M`.
    fn post_process<M: Component>(&mut self, post_process: impl ApplyPostProcess);

    /// Applies the post process to the camera with `M`, then binds the lifetime of the post process
    /// to the provided entity.
    fn bind_post_process<T: ApplyPostProcess + Sync, M: Component>(
        &mut self,
        post_process: T,
        entity: Entity,
    );

    /// Removes the post process from the camera with `M`.
    fn remove_post_process<T: ApplyPostProcess, M: Component>(&mut self);
}

impl PostProcessCommand for Commands<'_, '_> {
    fn post_process<M: Component>(&mut self, post_process: impl ApplyPostProcess) {
        self.queue(apply::<M>(post_process));
    }

    fn bind_post_process<T: ApplyPostProcess + Sync, M: Component>(
        &mut self,
        post_process: T,
        entity: Entity,
    ) {
        self.queue(bind::<T, M>(post_process, entity));
    }

    fn remove_post_process<T: ApplyPostProcess, M: Component>(&mut self) {
        self.queue(remove::<T, M>);
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

pub fn apply<M: Component>(post_process: impl ApplyPostProcess) -> impl FnOnce(&mut World) {
    move |world: &mut World| match world.query_filtered::<Entity, With<M>>().get_single(world) {
        Ok(camera) => {
            post_process.insert(&mut world.entity_mut(camera));
        }
        Err(e) => {
            error!("failed to apply post process to main camera: {e}");
        }
    }
}

struct PostProcessBinding<T, M>(PhantomData<T>, PhantomData<M>);

impl<T, M> Default for PostProcessBinding<T, M> {
    fn default() -> Self {
        Self(PhantomData, PhantomData)
    }
}

impl<T: ApplyPostProcess + Sync, M: Component> Component for PostProcessBinding<T, M> {
    const STORAGE_TYPE: StorageType = StorageType::Table;
    type Mutability = Mutable;

    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_remove(|mut world, _| {
            world.commands().queue(remove::<T, M>);
        });
    }
}

pub fn bind<T: ApplyPostProcess + Sync, M: Component>(
    post_process: T,
    entity: Entity,
) -> impl FnOnce(&mut World) {
    move |world: &mut World| match world.query_filtered::<Entity, With<M>>().get_single(world) {
        Ok(camera) => {
            post_process.insert(&mut world.entity_mut(camera));
            world
                .entity_mut(entity)
                .with_child(PostProcessBinding::<T, M>::default());
        }
        Err(e) => {
            error!("failed to bind post process to main camera: {e}");
        }
    }
}

pub fn remove<T: ApplyPostProcess, M: Component>(world: &mut World) {
    match world.query_filtered::<Entity, With<M>>().get_single(world) {
        Ok(camera) => {
            T::remove(&mut world.entity_mut(camera));
        }
        Err(e) => {
            error!("failed to remove post process from main camera: {e}");
        }
    }
}
