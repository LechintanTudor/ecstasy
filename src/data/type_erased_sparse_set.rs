use crate::data::{
    Component, ComponentFlags, Entity, SparseArray, SparseSetMutPtr, SparseSetRefMut, TypeErasedVec,
};

pub struct TypeErasedSparseSet {
    sparse: SparseArray,
    dense: Vec<Entity>,
    flags: Vec<ComponentFlags>,
    data: Box<dyn TypeErasedVec>,
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
            data: Box::new(Vec::<T>::new()),
        }
    }

    pub fn clear(&mut self) {
        self.sparse.clear();
        self.dense.clear();
        self.flags.clear();
        self.data.clear_components();
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        assert!(a != b);

        let sparse_index_a = self.dense[a].index();
        let sparse_index_b = self.dense[b].index();

        unsafe {
            self.sparse.swap_unchecked(sparse_index_a, sparse_index_b);
        }

        self.dense.swap(a, b);
        self.flags.swap(a, b);
        self.data.swap_components(a, b);
    }

    pub fn maintain(&mut self) {
        self.flags
            .iter_mut()
            .for_each(|flags| *flags = ComponentFlags::empty());
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.sparse.contains(entity)
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
                Box::as_mut(&mut self.data).downcast_mut().unwrap(),
            )
        }
    }

    pub fn to_mut_ptr<T>(&mut self) -> SparseSetMutPtr<T>
    where
        T: Component,
    {
        unsafe {
            SparseSetMutPtr::new(
                &mut self.sparse,
                &mut self.dense,
                &mut self.flags,
                Box::as_mut(&mut self.data).downcast_mut().unwrap(),
            )
        }
    }
}