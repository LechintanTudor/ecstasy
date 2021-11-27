use crate::components::{Component, ComponentGroupInfo, QueryGroupInfo};
use crate::query::{ChangeTicksFilter, ComponentRefMut, GetComponent};
use crate::storage::{ComponentStorage, Entity, EntitySparseArray};
use crate::utils::{ChangeTicks, Ticks};
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// View over a `ComponentStorage` of type `T`.
pub struct ComponentView<'a, T, S> {
    storage: S,
    group_info: Option<ComponentGroupInfo<'a>>,
    world_tick: Ticks,
    change_tick: Ticks,
    _phantom: PhantomData<*const T>,
}

impl<'a, T, S> ComponentView<'a, T, S>
where
    T: Component,
    S: Deref<Target = ComponentStorage>,
{
    pub(crate) unsafe fn new(
        storage: S,
        group_info: Option<ComponentGroupInfo<'a>>,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Self {
        Self {
            storage,
            group_info,
            world_tick,
            change_tick,
            _phantom: PhantomData,
        }
    }

    /// Returns the `ChangeTicks` of `entity`'s component.
    #[inline]
    pub fn get_ticks(&self, entity: Entity) -> Option<&ChangeTicks> {
        self.storage.get_ticks(entity)
    }

    /// Returns the component and `ChangeTicks` of `entity`.
    #[inline]
    pub fn get_with_ticks(&self, entity: Entity) -> Option<(&T, &ChangeTicks)> {
        unsafe { self.storage.get_with_ticks::<T>(entity) }
    }

    /// Returns the number of components in the view.
    #[inline]
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Returns `true` if the view is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Returns all entities in the view as a slice.
    #[inline]
    pub fn entities(&self) -> &[Entity] {
        self.storage.entities()
    }

    /// Returns all components in the view as a slice.
    #[inline]
    pub fn components(&self) -> &[T] {
        unsafe { self.storage.components::<T>() }
    }

    /// Returns all `ChangeTicks` in the view as a slice.
    #[inline]
    pub fn ticks(&self) -> &[ChangeTicks] {
        self.storage.ticks()
    }
}

impl<'a, T, S> fmt::Debug for ComponentView<'a, T, S>
where
    T: Component + fmt::Debug,
    S: Deref<Target = ComponentStorage>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entries = unsafe { self.storage.iter::<T>() };
        f.debug_list().entries(entries).finish()
    }
}

unsafe impl<'a, T, S> GetComponent<'a> for &'a ComponentView<'a, T, S>
where
    T: Component,
    S: Deref<Target = ComponentStorage>,
{
    type Item = &'a T;
    type Component = T;

    fn include_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>> {
        info.include(self.group_info?)
    }

    fn change_detection_ticks(&self) -> (Ticks, Ticks) {
        (self.world_tick, self.change_tick)
    }

    fn get_index(&self, entity: Entity) -> Option<usize> {
        self.storage.get_index_entity(entity).map(|e| e.dense())
    }

    unsafe fn get_unchecked<F>(self, index: usize) -> Option<Self::Item>
    where
        F: ChangeTicksFilter,
    {
        if F::IS_PASSTHROUGH {
            Some(self.storage.get_unchecked(index))
        } else {
            let (component, ticks) = self.storage.get_with_ticks_unchecked::<T>(index);

            if F::matches(ticks, self.world_tick, self.change_tick) {
                Some(component)
            } else {
                None
            }
        }
    }

    fn split(
        self,
    ) -> (
        &'a [Entity],
        &'a EntitySparseArray,
        *mut Self::Component,
        *mut ChangeTicks,
    ) {
        self.storage.split()
    }

    unsafe fn get_from_parts_unchecked<F>(
        components: *mut Self::Component,
        ticks: *mut ChangeTicks,
        index: usize,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>
    where
        F: ChangeTicksFilter,
    {
        if F::IS_PASSTHROUGH {
            Some(&*components.add(index))
        } else {
            let ticks = &*ticks.add(index);

            if F::matches(ticks, world_tick, change_tick) {
                Some(&*components.add(index))
            } else {
                None
            }
        }
    }
}

unsafe impl<'a, 'b, T, S> GetComponent<'a> for &'a mut ComponentView<'b, T, S>
where
    T: Component,
    S: Deref<Target = ComponentStorage> + DerefMut,
{
    type Item = ComponentRefMut<'a, T>;
    type Component = T;

    fn include_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>> {
        info.include(self.group_info?)
    }

    fn change_detection_ticks(&self) -> (Ticks, Ticks) {
        (self.world_tick, self.change_tick)
    }

    fn get_index(&self, entity: Entity) -> Option<usize> {
        self.storage.get_index_entity(entity).map(|e| e.dense())
    }

    unsafe fn get_unchecked<F>(self, index: usize) -> Option<Self::Item>
    where
        F: ChangeTicksFilter,
    {
        let (component, ticks) = self.storage.get_with_ticks_unchecked_mut::<T>(index);

        if F::IS_PASSTHROUGH {
            Some(ComponentRefMut::new(component, ticks, self.world_tick))
        } else {
            if F::matches(ticks, self.world_tick, self.change_tick) {
                Some(ComponentRefMut::new(component, ticks, self.world_tick))
            } else {
                None
            }
        }
    }

    fn split(
        self,
    ) -> (
        &'a [Entity],
        &'a EntitySparseArray,
        *mut Self::Component,
        *mut ChangeTicks,
    ) {
        self.storage.split()
    }

    unsafe fn get_from_parts_unchecked<F>(
        components: *mut Self::Component,
        ticks: *mut ChangeTicks,
        index: usize,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>
    where
        F: ChangeTicksFilter,
    {
        let component = &mut *components.add(index);
        let ticks = &mut *ticks.add(index);

        if F::IS_PASSTHROUGH {
            Some(ComponentRefMut::new(component, ticks, world_tick))
        } else {
            if F::matches(ticks, world_tick, change_tick) {
                Some(ComponentRefMut::new(component, ticks, world_tick))
            } else {
                None
            }
        }
    }
}
