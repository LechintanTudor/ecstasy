use crate::data::{
    Component, ComponentFlags, Entity, IndexEntity, SparseArray, SparseSetRef, SparseSetRefMut,
    TypeErasedVec,
};
use std::any::TypeId;

pub struct TypeErasedSparseSet {
    sparse: SparseArray,
    dense: Vec<Entity>,
    flags: Vec<ComponentFlags>,
    data: TypeErasedVec,
}

impl TypeErasedSparseSet {
    pub fn new<T>() -> Self
    where
        T: Component,
    {
        Self {
            sparse: Default::default(),
            dense: Default::default(),
            flags: Default::default(),
            data: TypeErasedVec::new::<T>(),
        }
    }

    pub fn component_type_id(&self) -> TypeId {
        self.data.type_info().id()
    }

    pub fn clear(&mut self) {
        self.sparse.clear();
        self.dense.clear();
        self.flags.clear();
        self.data.clear();
    }

    pub fn clear_flags(&mut self) {
        self.flags
            .iter_mut()
            .for_each(|flags| *flags = ComponentFlags::empty());
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }

        let sparse_index_a = self.dense[a].index();
        let sparse_index_b = self.dense[b].index();

        unsafe {
            self.sparse.swap_unchecked(sparse_index_a, sparse_index_b);
        }

        self.dense.swap(a, b);
        self.flags.swap(a, b);
        self.data.swap(a, b);
    }

    pub fn delete(&mut self, entity: Entity) {
        let index_entity = match self.sparse.get_index_entity(entity) {
            Some(index_entity) => index_entity,
            None => return,
        };

        let last_index = match self.dense.last() {
            Some(entity) => entity.index(),
            None => return,
        };

        self.dense.swap_remove(index_entity.index());
        self.flags.swap_remove(index_entity.index());

        unsafe {
            *self.sparse.get_unchecked_mut(last_index) = Some(index_entity);
            *self.sparse.get_unchecked_mut(entity.index()) = None;
        }

        self.data.swap_delete(index_entity.index());
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.sparse.contains(entity)
    }

    pub fn get_index_entity(&self, entity: Entity) -> Option<IndexEntity> {
        self.sparse.get_index_entity(entity)
    }

    pub fn to_ref<T>(&self) -> SparseSetRef<T>
    where
        T: Component,
    {
        unsafe { SparseSetRef::new(&self.sparse, &self.dense, &self.flags, self.data.as_ref()) }
    }

    pub fn to_ref_mut<T>(&mut self) -> SparseSetRefMut<T>
    where
        T: Component,
    {
        unsafe {
            SparseSetRefMut::new(
                &mut self.sparse,
                &mut self.dense,
                &mut self.flags,
                self.data.as_mut(),
            )
        }
    }
}