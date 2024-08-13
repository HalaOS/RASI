use std::collections::VecDeque;

use generic_array::{ArrayLength, GenericArray};

/// A key type trait used for [`KBucketTable`]
pub trait KBucketKey {
    type Length: ArrayLength;
    type Distance: KBucketDistance;

    /// Calculate the distance between two [`Key`]s.
    fn distance(&self, rhs: &Self) -> Self::Distance
    where
        Self: Sized;

    /// Returns the uniquely determined key with the given distance to `self`.
    ///
    /// This implements the following equivalence:
    ///
    /// `self xor other = distance <==> other = self xor distance`
    fn longest_common_prefix(&self, rhs: &Self) -> usize
    where
        Self: Sized;

    /// Returns the longest common prefix length with `rhs`.
    fn for_distance(&self, distance: Self::Distance) -> Self;
}

pub trait KBucketDistance {
    /// Returns the integer part of the base 2 logarithm of the [`Distance`].
    ///
    /// Returns `None` if the distance is zero.
    fn k_index(&self) -> Option<u32>;
}

struct KBucket<Key, Value>(VecDeque<(Key, Value)>);

impl<Key, Value> Default for KBucket<Key, Value> {
    fn default() -> Self {
        Self(VecDeque::default())
    }
}

impl<Key, Value> KBucket<Key, Value> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn remove(&mut self, remove_key: &Key) -> Option<(Key, Value)>
    where
        Key: PartialEq,
    {
        let mut remove_index = None;

        for (index, (key, _)) in self.0.iter().enumerate() {
            if *key == *remove_key {
                remove_index = Some(index);
                break;
            }
        }

        if let Some(remove_index) = remove_index {
            self.0.remove(remove_index)
        } else {
            None
        }
    }

    fn get(&self, key: &Key) -> Option<&Value>
    where
        Key: PartialEq,
    {
        for (k, value) in self.0.iter() {
            if *key == *k {
                return Some(value);
            }
        }

        None
    }
}

pub struct KBucketTable<Key, Value, const K: usize>
where
    Key: KBucketKey,
{
    local_key: Key,
    count: usize,
    buckets: Vec<KBucket<Key, Value>>,
    k_index: GenericArray<Option<usize>, Key::Length>,
}

impl<Key, Value, const K: usize> KBucketTable<Key, Value, K>
where
    Key: KBucketKey,
{
    /// Returns the const k value of the k-bucket.
    pub fn const_k() -> usize {
        K
    }
    /// Create a new KBucketsTable instance with local key value.
    pub fn new(local_key: Key) -> Self {
        assert!(K > 0, "the k must greater than zero");
        Self {
            local_key,
            count: 0,
            buckets: Default::default(),
            k_index: Default::default(),
        }
    }

    /// Return the parameter `K` of k-bucket.
    pub fn k(&self) -> usize {
        K
    }

    /// Returns the local node key.
    pub fn local_key(&self) -> &Key {
        &self.local_key
    }

    /// Get the count of stored keys.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns value of given key.
    pub fn get(&self, key: &Key) -> Option<&Value>
    where
        Key: PartialEq,
    {
        let k_index = self.k_index_of(key);

        if let Some(bucket) = self.bucket(k_index) {
            bucket.get(key)
        } else {
            None
        }
    }

    /// Push value into k-bucket's bottom. and pop the LRU value from the k-bucket's top if the k-bucket is full.
    ///
    /// This function has no side effects if called with `local_key`.
    pub fn insert(&mut self, key: Key, value: Value) -> Option<(Key, Value)>
    where
        Key: Clone + PartialEq,
    {
        let k_index = self.k_index_of(&key);

        let k_bucket = if let Some(index) = self.k_index[k_index] {
            self.buckets.get_mut(index).unwrap()
        } else {
            self.buckets.push(KBucket::default());
            self.k_index[k_index] = Some(self.buckets.len() - 1);
            self.buckets.last_mut().unwrap()
        };

        // the key is in the k-bucket, replace old value.
        if let Some((key, _)) = k_bucket.remove(&key) {
            k_bucket.0.push_back((key, value));
            return None;
        }

        // k-bucket is full
        if k_bucket.0.len() == K {
            let removed = k_bucket.0.pop_front().expect("K > 0");

            k_bucket.0.push_back((key, value));

            return Some(removed);
        } else {
            k_bucket.0.push_back((key.clone(), value));

            self.count += 1;

            return None;
        }
    }

    /// Returns an iterator of up to `k` keys closest to `target`.
    pub fn closest_k(&self, target: &Key) -> KBucketTableIter<'_, Key, Value, K> {
        let k_index = self.k_index_of(target);

        let bucket_len = if let Some(bucket) = self.bucket(k_index) {
            if bucket.len() == K {
                return KBucketTableIter {
                    table: self,
                    k_offset: k_index,
                    k_end_offset: k_index,
                    k_inner_offset: 0,
                    k_end_inner_offset: K,
                };
            }

            bucket.len()
        } else {
            0
        };

        let mut k_offset = k_index;
        let mut k_end_offset = k_index;

        let mut k_inner_offset = 0;
        let mut k_end_inner_offset: usize = bucket_len;

        let mut nodes = bucket_len;

        while nodes < K {
            if k_offset > 0 {
                k_offset -= 1;
                if let Some(bucket) = self.bucket(k_offset) {
                    nodes += bucket.len();

                    if nodes >= K {
                        k_inner_offset = nodes - K;
                        // nodes = self.const_k;
                        break;
                    } else {
                        k_inner_offset = 0;
                    }
                }
            }

            if k_end_offset < self.k_index.len() {
                k_end_offset += 1;

                if let Some(bucket) = self.bucket(k_offset) {
                    nodes += bucket.len();
                    k_end_inner_offset = bucket.len();

                    if nodes >= K {
                        k_end_inner_offset -= nodes - K;
                        // nodes = self.const_k;
                        break;
                    }
                }
            } else {
                break;
            }
        }

        return KBucketTableIter {
            table: self,
            k_offset,
            k_end_offset,
            k_inner_offset,
            k_end_inner_offset,
        };
    }

    /// Returns an iterator over all inserted keys.
    pub fn iter(&self) -> KBucketTableIter<'_, Key, Value, K> {
        let k_end_inner_offset = self
            .bucket(self.k_index.len())
            .map(|bucket| bucket.len())
            .unwrap_or(0);

        return KBucketTableIter {
            table: self,
            k_offset: 0,
            k_end_offset: self.k_index.len(),
            k_inner_offset: 0,
            k_end_inner_offset,
        };
    }

    fn k_index_of(&self, key: &Key) -> usize {
        let k_index = key
            .distance(&self.local_key)
            .k_index()
            .expect("Get local key index") as usize;

        assert!(k_index < self.k_index.len());

        k_index
    }

    fn bucket(&self, index: usize) -> Option<&KBucket<Key, Value>> {
        self.k_index[index].map(|index| &self.buckets[index])
    }
}

/// An immutable iterator over [`KBucketsTable`]
pub struct KBucketTableIter<'a, Key, Value, const K: usize>
where
    Key: KBucketKey,
{
    table: &'a KBucketTable<Key, Value, K>,
    k_offset: usize,
    k_inner_offset: usize,
    k_end_offset: usize,
    k_end_inner_offset: usize,
}

impl<'a, Key, Value, const K: usize> Iterator for KBucketTableIter<'a, Key, Value, K>
where
    Key: KBucketKey,
{
    type Item = &'a (Key, Value);

    fn next(&mut self) -> Option<Self::Item> {
        if self.k_offset == self.k_end_offset && self.k_inner_offset == self.k_end_inner_offset {
            return None;
        }

        let k_bucket_offset = self.table.k_index[self.k_offset].expect("k-bucket not exists");

        let k_bucket = &self.table.buckets[k_bucket_offset];

        //  In the bucket, iterate in MRU order
        let item = &k_bucket.0[k_bucket.0.len() - self.k_inner_offset - 1];

        self.k_inner_offset += 1;

        if self.k_inner_offset == k_bucket.0.len() {
            self.k_inner_offset = 0;
            self.k_offset += 1;
        }

        Some(item)
    }
}