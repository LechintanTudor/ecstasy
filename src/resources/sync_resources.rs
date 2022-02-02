use crate::resources::{Res, ResMut, Resource, UnsafeResources};

#[derive(Clone, Copy)]
pub struct SyncResources<'a> {
    resources: &'a UnsafeResources,
}

unsafe impl Send for SyncResources<'_> {}
unsafe impl Sync for SyncResources<'_> {}

impl<'a> SyncResources<'a> {
    pub(crate) fn new(resources: &'a UnsafeResources) -> Self {
        Self { resources }
    }

    pub fn borrow<T>(&self) -> Option<Res<'a, T>>
    where
        T: Resource + Sync,
    {
        unsafe { self.resources.borrow::<T>().map(Res::new) }
    }

    pub fn borrow_mut<T>(&self) -> Option<ResMut<'a, T>>
    where
        T: Resource + Send,
    {
        unsafe { self.resources.borrow_mut::<T>().map(ResMut::new) }
    }
}