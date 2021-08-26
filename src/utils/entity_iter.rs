use crate::storage::Entity;

pub trait EntityIterator
where
	Self: Iterator + Sized,
{
	fn current_entity(&self) -> Option<Entity>;

	fn entities(self) -> EntityIter<Self> {
		EntityIter(self)
	}
}

pub struct EntityIter<I>(I);

impl<I> Iterator for EntityIter<I>
where
	I: EntityIterator,
{
	type Item = (Entity, I::Item);

	fn next(&mut self) -> Option<Self::Item> {
		Some((self.0.current_entity()?, self.0.next()?))
	}
}