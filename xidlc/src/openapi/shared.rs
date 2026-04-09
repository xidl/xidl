use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error, Expected, Visitor},
};
use std::fmt::Formatter;

use super::Ref;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Number {
    Int(isize),
    UInt(usize),
    Float(f64),
}

impl Eq for Number {}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::UInt(left), Self::UInt(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            _ => false,
        }
    }
}

macro_rules! impl_from_for_number {
    ( $( $ty:ident => $pat:ident $( as $as:ident )? ),* ) => {
        $(
        impl From<$ty> for Number {
            fn from(value: $ty) -> Self {
                Self::$pat(value $( as $as )?)
            }
        }
        )*
    };
}

#[rustfmt::skip]
impl_from_for_number!(
    f32 => Float as f64, f64 => Float,
    i8 => Int as isize, i16 => Int as isize, i32 => Int as isize, i64 => Int as isize,
    u8 => UInt as usize, u16 => UInt as usize, u32 => UInt as usize, u64 => UInt as usize,
    isize => Int, usize => UInt
);

#[derive(Serialize, Clone, PartialEq, Eq, Default)]
pub enum OpenApiVersion {
    #[serde(rename = "3.1.0")]
    #[default]
    Version31,
}

impl<'de> Deserialize<'de> for OpenApiVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VersionVisitor;

        impl<'v> Visitor<'v> for VersionVisitor {
            type Value = OpenApiVersion;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a version string in 3.1.x format")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.visit_string(v.to_string())
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let version = v
                    .split('.')
                    .flat_map(|digit| digit.parse::<i8>())
                    .collect::<Vec<_>>();

                if version.len() == 3 && version.first() == Some(&3) && version.get(1) == Some(&1) {
                    Ok(OpenApiVersion::Version31)
                } else {
                    let expected: &dyn Expected = &"3.1.0";
                    Err(Error::invalid_value(
                        serde::de::Unexpected::Str(&v),
                        expected,
                    ))
                }
            }
        }

        deserializer.deserialize_string(VersionVisitor)
    }
}

#[derive(PartialEq, Eq, Clone, Default)]
#[allow(missing_docs)]
pub enum Deprecated {
    True,
    #[default]
    False,
}

impl Serialize for Deprecated {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(matches!(self, Self::True))
    }
}

impl<'de> Deserialize<'de> for Deprecated {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_bool_flag(deserializer, Deprecated::True, Deprecated::False)
    }
}

#[derive(PartialEq, Eq, Clone, Default)]
#[allow(missing_docs)]
pub enum Required {
    True,
    #[default]
    False,
}

impl Serialize for Required {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(matches!(self, Self::True))
    }
}

impl<'de> Deserialize<'de> for Required {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_bool_flag(deserializer, Required::True, Required::False)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum RefOr<T> {
    Ref(Ref),
    T(T),
}

fn deserialize_bool_flag<'de, D, T>(
    deserializer: D,
    true_value: T,
    false_value: T,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Clone,
{
    struct BoolVisitor<T> {
        true_value: T,
        false_value: T,
    }

    impl<'de, T: Clone> Visitor<'de> for BoolVisitor<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a bool true or false")
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(if v { self.true_value } else { self.false_value })
        }
    }

    deserializer.deserialize_bool(BoolVisitor {
        true_value,
        false_value,
    })
}
