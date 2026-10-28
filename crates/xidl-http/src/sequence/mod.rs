//! Bounded sequence validation at serialization boundaries.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, ops::Deref};

/// A wire sequence with an inclusive maximum length.
///
/// Owned conversions may collect unchecked domain values. Serialization and
/// deserialization always check the bound; `TryFrom<Vec<T>>` checks immediately
/// for transports that handle raw bytes or metadata without serde.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceWire<T, const MAX: usize>(Vec<T>);

/// A sequence exceeds its declared IDL maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceLimit {
    /// Number of supplied elements.
    pub actual: usize,
    /// Inclusive maximum from the IDL contract.
    pub maximum: usize,
}

impl fmt::Display for SequenceLimit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "sequence length {} exceeds maximum {}",
            self.actual, self.maximum
        )
    }
}

impl std::error::Error for SequenceLimit {}

impl SequenceLimit {
    /// Checks a wire collection length without copying its elements.
    pub fn check(actual: usize, maximum: usize) -> Result<(), Self> {
        if actual > maximum {
            Err(Self { actual, maximum })
        } else {
            Ok(())
        }
    }
}

impl<T, const MAX: usize> SequenceWire<T, MAX> {
    /// Validates the length before a non-serde wire operation.
    pub fn validate(&self) -> Result<(), SequenceLimit> {
        SequenceLimit::check(self.0.len(), MAX)
    }
}

impl<T, const MAX: usize> TryFrom<Vec<T>> for SequenceWire<T, MAX> {
    type Error = SequenceLimit;

    fn try_from(values: Vec<T>) -> Result<Self, Self::Error> {
        let wire = Self(values);
        wire.validate()?;
        Ok(wire)
    }
}

impl<T, const MAX: usize> Default for SequenceWire<T, MAX> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T, const MAX: usize> Deref for SequenceWire<T, MAX> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const MAX: usize> FromIterator<T> for SequenceWire<T, MAX> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T, const MAX: usize> IntoIterator for SequenceWire<T, MAX> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T, const MAX: usize> IntoIterator for &'a SequenceWire<T, MAX> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T: Serialize, const MAX: usize> Serialize for SequenceWire<T, MAX> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.validate().map_err(serde::ser::Error::custom)?;
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>, const MAX: usize> Deserialize<'de> for SequenceWire<T, MAX> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::try_from(Vec::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
