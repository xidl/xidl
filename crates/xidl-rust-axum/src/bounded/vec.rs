use super::BoundExceeded;
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{fmt, marker::PhantomData, ops::Deref};

/// A sequence containing at most `MAX` elements, serialized as an array.
///
/// Construction and growth reject overflow without truncating values. Mutable
/// access is limited to a slice so it cannot invalidate the length constraint.
///
/// ```
/// use xidl_rust_axum::BoundedVec;
/// let mut batch = BoundedVec::<_, 2>::try_from(vec![10])?;
/// batch.try_push(20)?;
/// assert!(batch.try_push(30).is_err());
/// # Ok::<(), xidl_rust_axum::BoundExceeded>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct BoundedVec<T, const MAX: usize>(Vec<T>);

impl<T, const MAX: usize> BoundedVec<T, MAX> {
    /// Constructs an empty bounded sequence.
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Collects values, rejecting the first element beyond the maximum.
    pub fn try_from_iter(values: impl IntoIterator<Item = T>) -> Result<Self, BoundExceeded> {
        let mut result = Self::new();
        for value in values {
            result.try_push(value)?;
        }
        Ok(result)
    }

    /// Appends one element, leaving the sequence unchanged on overflow.
    pub fn try_push(&mut self, value: T) -> Result<(), BoundExceeded> {
        BoundExceeded::check("sequence", self.0.len().saturating_add(1), MAX)?;
        self.0.push(value);
        Ok(())
    }

    /// Removes the last element, releasing one slot for subsequent insertion.
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    /// Borrows elements for modification without allowing the length to change.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.0
    }

    /// Consumes the sequence, returning its elements without copying.
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }

    /// Transforms each element while preserving the maximum and element count.
    pub fn map<U>(self, convert: impl FnMut(T) -> U) -> BoundedVec<U, MAX> {
        BoundedVec(self.0.into_iter().map(convert).collect())
    }
}

impl<T, const MAX: usize> Default for BoundedVec<T, MAX> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const MAX: usize> TryFrom<Vec<T>> for BoundedVec<T, MAX> {
    type Error = BoundExceeded;

    fn try_from(values: Vec<T>) -> Result<Self, Self::Error> {
        BoundExceeded::check("sequence", values.len(), MAX)?;
        Ok(Self(values))
    }
}

impl<T, const MAX: usize> From<BoundedVec<T, MAX>> for Vec<T> {
    fn from(value: BoundedVec<T, MAX>) -> Self {
        value.into_inner()
    }
}

impl<T, const MAX: usize> Deref for BoundedVec<T, MAX> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const MAX: usize> AsRef<[T]> for BoundedVec<T, MAX> {
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<T, const MAX: usize> IntoIterator for BoundedVec<T, MAX> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T, const MAX: usize> IntoIterator for &'a BoundedVec<T, MAX> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'de, T: Deserialize<'de>, const MAX: usize> Deserialize<'de> for BoundedVec<T, MAX> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<T, const MAX: usize>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>, const MAX: usize> de::Visitor<'de> for Visitor<T, MAX> {
            type Value = BoundedVec<T, MAX>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a sequence with at most {MAX} elements")
            }

            fn visit_seq<A: de::SeqAccess<'de>>(
                self,
                mut input: A,
            ) -> Result<Self::Value, A::Error> {
                let mut result = BoundedVec::new();
                while let Some(value) = input.next_element()? {
                    result.try_push(value).map_err(de::Error::custom)?;
                }
                Ok(result)
            }
        }

        deserializer.deserialize_seq(Visitor::<T, MAX>(PhantomData))
    }
}
