use indexmap::IndexMap;
use std::hash::Hash;

pub trait IteratorExt: Iterator {
    fn ordered_group_by<K, F>(self, mut key: F) -> IndexMap<K, Vec<Self::Item>>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> K,
        K: PartialEq + Eq + Hash,
    {
        let mut result = IndexMap::new();

        for i in self.into_iter() {
            let items = result.entry(key(&i)).or_insert_with(Vec::new);
            items.push(i);
        }

        result
    }
}

impl<T: ?Sized> IteratorExt for T where T: Iterator {}
