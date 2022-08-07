//! Singly linked lists
//!
//! It's highly unlikely that the user machine contains more than a million of
//! processors, so we use linked lists to [minimize the code size][1] at the
//! cost of scalability. Besides, linked lists are fun!
//!
//! [1]: https://rust.godbolt.org/z/78TnvfqYs

/// A singly-linked list entry
#[derive(Debug, Clone)]
pub struct Entry<T> {
    pub data: T,
    pub next: Link<T>,
}

/// An optional reference to an [`Entry`].
pub type Link<T> = Option<Box<Entry<T>>>;

#[inline]
pub fn clear<T>(head: &mut Link<T>) {
    while let Some(entry) = head.take() {
        *head = entry.next;
    }
}

#[inline]
pub fn iter<T>(mut head: &Link<T>) -> impl Iterator<Item = &T> + '_ {
    std::iter::from_fn(move || {
        if let Some(entry) = head.as_deref() {
            head = &entry.next;
            Some(&entry.data)
        } else {
            None
        }
    })
}

#[inline]
pub fn iter_mut<T>(head: &mut Link<T>) -> impl Iterator<Item = &mut T> + '_ {
    // allow the closure to temporarily move out from `head`
    let mut head = Some(head);
    std::iter::from_fn(move || {
        if let Some(entry) = head.take()?.as_deref_mut() {
            head = Some(&mut entry.next);
            Some(&mut entry.data)
        } else {
            None
        }
    })
}

#[inline]
pub fn resize_with<T>(mut head: &mut Link<T>, len: usize, mut f: impl FnMut(usize) -> T) {
    let mut i = 0;
    while i < len {
        head = &mut head
            .get_or_insert_with(|| {
                Box::new(Entry {
                    data: f(i),
                    next: None,
                })
            })
            .next;
        i += 1;
    }

    // Drop excessive entries
    clear(head);
}

/// Iterate through `I` while calling `mutate` for each element in `head` in a
/// lockstep. Missing elements are created on-the-fly by `new` before calling
/// `mutate`. Any excess elements are removed after successfully processing the
/// first `iter.count()` elements.
#[inline]
pub fn try_for_each_and_resize<T, R, I: Iterator>(
    mut head: &mut Link<T>,
    iter: I,
    mut new: impl FnMut(&mut I::Item) -> T,
    mut mutate: impl FnMut(&mut T, I::Item) -> Result<(), R>,
) -> Result<(), R> {
    for mut i in iter {
        let cur = head.get_or_insert_with(|| {
            Box::new(Entry {
                data: new(&mut i),
                next: None,
            })
        });
        mutate(&mut cur.data, i)?;
        head = &mut cur.next;
    }

    // Drop excessive entries
    clear(head);

    Ok(())
}
