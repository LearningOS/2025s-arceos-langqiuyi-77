use core::ops::{Deref, DerefMut};
use arceos_api::random_u64;
use hashbrown::HashMap as InnerHashMap;

use super::simple_hasher::SimpleBuildHasher;

pub struct HashMap<K, V> {
    inner: InnerHashMap<K, V, SimpleBuildHasher>,
}

impl<K, V> HashMap<K, V> {
    pub fn new() -> Self {
        let seed = random_u64();
        let hasher = SimpleBuildHasher::new(seed);
        let inner = InnerHashMap::with_hasher(hasher);
        HashMap { inner }
    }
}

// 让你可以用 map.insert() / map.get() 等所有方法
impl<K, V> Deref for HashMap<K, V> {
    type Target = InnerHashMap<K, V, SimpleBuildHasher>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K, V> DerefMut for HashMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
