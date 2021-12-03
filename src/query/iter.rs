use crate::query::{is_trivial_group, DenseIter, QueryFilter, QueryGet, QueryModifier, SparseIter};
use crate::storage::Entity;
use crate::utils::EntityIterator;

/// Iterator over grouped or ungrouped queries.
pub enum Iter<'a, G, I, E, F>
where
    G: QueryGet<'a>,
    I: QueryModifier<'a>,
    E: QueryModifier<'a>,
    F: QueryFilter,
{
    /// Iterator over ungrouped queries.
    Sparse(SparseIter<'a, G, I, E, F>),
    /// Iterator over grouped queries. Extremely fast.
    Dense(DenseIter<'a, G, F>),
}

impl<'a, G, I, E, F> Iter<'a, G, I, E, F>
where
    G: QueryGet<'a>,
    I: QueryModifier<'a>,
    E: QueryModifier<'a>,
    F: QueryFilter,
{
    /// Creates a new iterator from the given `Query` parts.
    pub(crate) fn new(get: G, include: I, exclude: E, filter: F) -> Self {
        let (world_tick, change_tick) = get.change_detection_ticks();

        // if query::is_trivial_group::<G, I, E>() {
        //     let (iter_data, data) = base.split_dense();

        //     unsafe { Self::Dense(DenseIter::new_unchecked(iter_data, data, filter)) }
        // } else {
        //     match query::group_range(&base, &include, &exclude) {
        //         Ok(range) => {
        //             let (mut iter_data, data) = base.split_dense();

        //             unsafe {
        //                 iter_data.entities = iter_data.entities.get_unchecked(range);
        //                 Self::Dense(DenseIter::new_unchecked(iter_data, data,
        // filter))             }
        //         }
        //         Err(_) => {
        //             let (mut iter_data, sparse, data) = base.split_sparse();
        //             let (include_entities, include) = include.split_modifier();
        //             let (_, exclude) = exclude.split_modifier();

        //             if let Some(entities) = include_entities {
        //                 if entities.len() < iter_data.entities.len() {
        //                     iter_data.entities = entities;
        //                 }
        //             }

        //             Self::Sparse(SparseIter::new(
        //                 iter_data, sparse, data, include, exclude, filter,
        //             ))
        //         }
        //     }
        // }
        todo!()
    }

    /// Returns `true` if the iterator is dense.
    pub fn is_dense(&self) -> bool {
        matches!(self, Self::Dense(_))
    }
}

impl<'a, G, I, E, F> Iterator for Iter<'a, G, I, E, F>
where
    G: QueryGet<'a>,
    I: QueryModifier<'a>,
    E: QueryModifier<'a>,
    F: QueryFilter,
{
    type Item = G::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Sparse(sparse) => sparse.next(),
            Self::Dense(dense) => dense.next(),
        }
    }

    fn fold<Acc, Func>(self, init: Acc, f: Func) -> Acc
    where
        Self: Sized,
        Func: FnMut(Acc, Self::Item) -> Acc,
    {
        match self {
            Self::Sparse(sparse) => sparse.fold(init, f),
            Self::Dense(dense) => dense.fold(init, f),
        }
    }
}

unsafe impl<'a, G, I, E, F> EntityIterator for Iter<'a, G, I, E, F>
where
    G: QueryGet<'a>,
    I: QueryModifier<'a>,
    E: QueryModifier<'a>,
    F: QueryFilter,
{
    fn next_with_entity(&mut self) -> Option<(Entity, Self::Item)> {
        match self {
            Self::Sparse(sparse) => sparse.next_with_entity(),
            Self::Dense(dense) => dense.next_with_entity(),
        }
    }

    fn fold_with_entity<Acc, Func>(self, init: Acc, f: Func) -> Acc
    where
        Self: Sized,
        Func: FnMut(Acc, (Entity, Self::Item)) -> Acc,
    {
        match self {
            Self::Sparse(sparse) => sparse.fold_with_entity(init, f),
            Self::Dense(dense) => dense.fold_with_entity(init, f),
        }
    }
}
