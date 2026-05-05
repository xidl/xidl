use std::{collections::BTreeMap, iter};

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Scopes {
    scopes: BTreeMap<String, String>,
}

impl Scopes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn one<S: Into<String>>(scope: S, description: S) -> Self {
        Self {
            scopes: BTreeMap::from_iter(iter::once_with(|| (scope.into(), description.into()))),
        }
    }
}

impl<I> FromIterator<(I, I)> for Scopes
where
    I: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (I, I)>>(iter: T) -> Self {
        Self {
            scopes: iter
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }
}
