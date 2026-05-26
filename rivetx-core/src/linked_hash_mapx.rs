use linked_hash_map::LinkedHashMap;
use std::collections::hash_map::{self};
use std::hash::{BuildHasher, Hash};
use linked_hash_map::Entry;

pub struct LinkedHashMapx<K, V, S = hash_map::RandomState> {
    pub max_size: usize,
    pub hash_map: LinkedHashMap<K, V, S>,
}
impl<K: Hash + Eq, V> LinkedHashMapx<K, V> {
    pub fn new(max_size: usize) -> Self {
        return Self{
             max_size,
             hash_map: LinkedHashMap::new(),
        }
    }

    pub fn with_capacity(max_size: usize, capacity: usize) -> Self {
        return Self{
            max_size,
            hash_map: LinkedHashMap::with_capacity(capacity),
        }
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> LinkedHashMapx<K, V, S> {
    //max_size只支持insert函数
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let value = self.hash_map.insert(k, v);
        if self.max_size > 0 && self.hash_map.len() > self.max_size {
            self.hash_map.pop_front();
        }
        value
    }

    pub fn try_insert(&mut self, k: K, v: V) -> bool {
        let entry = self.hash_map.entry(k);
        match entry {
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(v);

                if self.max_size > 0 && self.hash_map.len() > self.max_size {
                    self.hash_map.pop_front();
                }
                true
            }
            Entry::Occupied(_) => false,
        }
    }
}


