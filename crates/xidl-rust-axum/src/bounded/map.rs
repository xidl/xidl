use super::BoundExceeded;
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{borrow::Borrow, collections::BTreeMap, fmt, marker::PhantomData, ops::Deref};

/// An ordered map containing at most `MAX` entries, serialized as an object.
///
/// Replacing an existing key is allowed at capacity. No mutable reference to the
/// underlying map is exposed, so growth always preserves the bound.
///
/// ```
/// use xidl_rust_axum::BoundedMap;
/// let mut regions = BoundedMap::<_, _, 1>::try_from_iter([("eu", 10)])?;
/// assert_eq!(regions.try_insert("eu", 20)?, Some(10));
/// assert!(regions.try_insert("us", 30).is_err());
/// # Ok::<(), xidl_rust_axum::BoundExceeded>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct BoundedMap<K, V, const MAX: usize>(BTreeMap<K, V>);

impl<K, V, const MAX: usize> BoundedMap<K, V, MAX> {
    /// Constructs an empty bounded map.
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Consumes the map, returning its entries without copying.
    pub fn into_inner(self) -> BTreeMap<K, V> {
        self.0
    }

    /// Transforms values while preserving every key and the entry count.
    pub fn map_values<W>(self, mut convert: impl FnMut(V) -> W) -> BoundedMap<K, W, MAX>
    where
        K: Ord,
    {
        BoundedMap(
            self.0
                .into_iter()
                .map(|(key, value)| (key, convert(value)))
                .collect(),
        )
    }
}

impl<K: Ord, V, const MAX: usize> BoundedMap<K, V, MAX> {
    /// Collects entries using the same replacement semantics as `BTreeMap`.
    pub fn try_from_iter(values: impl IntoIterator<Item = (K, V)>) -> Result<Self, BoundExceeded> {
        let mut result = Self::new();
        for (key, value) in values {
            result.try_insert(key, value)?;
        }
        Ok(result)
    }

    /// Inserts or replaces a value, leaving the map unchanged on overflow.
    pub fn try_insert(&mut self, key: K, value: V) -> Result<Option<V>, BoundExceeded> {
        if !self.0.contains_key(&key) {
            BoundExceeded::check("map", self.0.len().saturating_add(1), MAX)?;
        }
        Ok(self.0.insert(key, value))
    }

    /// Borrows an existing value without allowing the entry count to change.
    pub fn get_mut<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        self.0.get_mut(key)
    }

    /// Removes an entry, releasing one slot for subsequent insertion.
    pub fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.0.remove(key)
    }
}

impl<K, V, const MAX: usize> Default for BoundedMap<K, V, MAX> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, const MAX: usize> TryFrom<BTreeMap<K, V>> for BoundedMap<K, V, MAX> {
    type Error = BoundExceeded;

    fn try_from(values: BTreeMap<K, V>) -> Result<Self, Self::Error> {
        BoundExceeded::check("map", values.len(), MAX)?;
        Ok(Self(values))
    }
}

impl<K, V, const MAX: usize> From<BoundedMap<K, V, MAX>> for BTreeMap<K, V> {
    fn from(value: BoundedMap<K, V, MAX>) -> Self {
        value.into_inner()
    }
}

impl<K, V, const MAX: usize> Deref for BoundedMap<K, V, MAX> {
    type Target = BTreeMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V, const MAX: usize> IntoIterator for BoundedMap<K, V, MAX> {
    type Item = (K, V);
    type IntoIter = std::collections::btree_map::IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K, V, const MAX: usize> IntoIterator for &'a BoundedMap<K, V, MAX> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::collections::btree_map::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'de, K: Ord + Deserialize<'de>, V: Deserialize<'de>, const MAX: usize> Deserialize<'de>
    for BoundedMap<K, V, MAX>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<K, V, const MAX: usize>(PhantomData<(K, V)>);

        impl<'de, K: Ord + Deserialize<'de>, V: Deserialize<'de>, const MAX: usize> de::Visitor<'de>
            for Visitor<K, V, MAX>
        {
            type Value = BoundedMap<K, V, MAX>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a map with at most {MAX} entries")
            }

            fn visit_map<A: de::MapAccess<'de>>(
                self,
                mut input: A,
            ) -> Result<Self::Value, A::Error> {
                let mut result = BoundedMap::new();
                while let Some((key, value)) = input.next_entry()? {
                    result.try_insert(key, value).map_err(de::Error::custom)?;
                }
                Ok(result)
            }
        }

        deserializer.deserialize_map(Visitor::<K, V, MAX>(PhantomData))
    }
}
