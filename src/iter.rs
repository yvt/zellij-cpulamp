/// [`Iterator`] with a smaller vtable
pub trait MiniIterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

pub type BoxMiniIterator<'a, Item> = Box<dyn MiniIterator<Item = Item> + 'a>;

impl<T: Iterator> MiniIterator for T {
    type Item = <Self as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        Iterator::next(self)
    }
}

impl<Item> Iterator for dyn MiniIterator<Item = Item> + '_ {
    type Item = Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        (*self).next()
    }
}
