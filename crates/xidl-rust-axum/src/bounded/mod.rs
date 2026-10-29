//! Containers that preserve IDL maximum lengths throughout their lifetime.

mod map;
mod vec;

pub use map::BoundedMap;
pub use vec::BoundedVec;

/// A collection exceeds the maximum declared by its IDL contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{collection} length {actual} exceeds maximum {maximum}")]
pub struct BoundExceeded {
    /// Collection whose construction or modification failed.
    pub collection: &'static str,
    /// Number of elements the operation would produce.
    pub actual: usize,
    /// Inclusive maximum number of elements.
    pub maximum: usize,
}

impl BoundExceeded {
    fn check(collection: &'static str, actual: usize, maximum: usize) -> Result<(), Self> {
        if actual > maximum {
            Err(Self {
                collection,
                actual,
                maximum,
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
