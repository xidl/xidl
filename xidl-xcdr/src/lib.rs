pub mod error;

pub mod cdr;
pub mod delimited_cdr;
pub mod ffi;
pub mod plain_cdr;
pub mod plain_cdr2;
pub mod plcdr;
pub mod plcdr2;
pub mod xcdr_plcdr;

mod utils;

use error::{XcdrError, XcdrResult};
use std::collections::BTreeMap;

pub struct FieldId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializeKind {
    Cdr,
    PlainCdr,
    PlCdr,
    PlainCdr2,
    DelimitedCdr,
    PlCdr2,
}

#[allow(unused_variables)]
pub trait XcdrSerializer {
    fn begin_struct(&mut self) -> XcdrResult<()> {
        Ok(())
    }
    fn end_struct(&mut self) -> XcdrResult<()> {
        Ok(())
    }

    fn begin_field(
        &mut self,
        id: FieldId,
        must_understand: bool,
        length_code: u8,
    ) -> XcdrResult<()> {
        Ok(())
    }
    fn end_field(&mut self) -> XcdrResult<()> {
        Ok(())
    }

    fn write_bool(&mut self, val: bool) -> XcdrResult<()>;
    fn write_u8(&mut self, val: u8) -> XcdrResult<()>;
    fn write_i8(&mut self, val: i8) -> XcdrResult<()>;
    fn write_u16(&mut self, val: u16) -> XcdrResult<()>;
    fn write_i16(&mut self, val: i16) -> XcdrResult<()>;
    fn write_u32(&mut self, val: u32) -> XcdrResult<()>;
    fn write_i32(&mut self, val: i32) -> XcdrResult<()>;
    fn write_u64(&mut self, val: u64) -> XcdrResult<()>;
    fn write_i64(&mut self, val: i64) -> XcdrResult<()>;
    fn write_f32(&mut self, val: f32) -> XcdrResult<()>;
    fn write_f64(&mut self, val: f64) -> XcdrResult<()>;
    fn write_bytes(&mut self, buf: &[u8]) -> XcdrResult<()>;

    fn write<T: XcdrSerialize>(&mut self, value: &T) -> XcdrResult<()> {
        value.serialize_with(self)
    }
}

pub trait XcdrDeserializer {
    fn next_field(&mut self) -> XcdrResult<Option<FieldId>> {
        Ok(None)
    }
    fn enter_struct(&mut self) -> XcdrResult<()> {
        Ok(())
    }
    fn exit_struct(&mut self) -> XcdrResult<()> {
        Ok(())
    }
    fn skip_field(&mut self) -> XcdrResult<()> {
        Ok(())
    }

    fn read_u8(&mut self) -> XcdrResult<u8>;
    fn read_i8(&mut self) -> XcdrResult<i8>;
    fn read_bool(&mut self) -> XcdrResult<bool>;
    fn read_u16_le(&mut self) -> XcdrResult<u16>;
    fn read_u16_be(&mut self) -> XcdrResult<u16>;
    fn read_i16_le(&mut self) -> XcdrResult<i16>;
    fn read_i16_be(&mut self) -> XcdrResult<i16>;
    fn read_u32_le(&mut self) -> XcdrResult<u32>;
    fn read_u32_be(&mut self) -> XcdrResult<u32>;
    fn read_i32_le(&mut self) -> XcdrResult<i32>;
    fn read_i32_be(&mut self) -> XcdrResult<i32>;
    fn read_u64_le(&mut self) -> XcdrResult<u64>;
    fn read_u64_be(&mut self) -> XcdrResult<u64>;
    fn read_i64_le(&mut self) -> XcdrResult<i64>;
    fn read_i64_be(&mut self) -> XcdrResult<i64>;
    fn read_f32_le(&mut self) -> XcdrResult<f32>;
    fn read_f32_be(&mut self) -> XcdrResult<f32>;
    fn read_f64_le(&mut self) -> XcdrResult<f64>;
    fn read_f64_be(&mut self) -> XcdrResult<f64>;
    fn read_bytes(&mut self, out: &mut [u8]) -> XcdrResult<()>;

    fn read<T: XcdrDeserialize>(&mut self) -> XcdrResult<T> {
        T::deserialize(self)
    }
}

pub trait XcdrSerialize {
    fn serialize_kind(&self) -> SerializeKind {
        SerializeKind::Cdr
    }

    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, serializer: &mut S) -> XcdrResult<()>;

    fn serialize(&self, buf: &mut [u8]) -> XcdrResult<usize>
    where
        Self: Sized,
    {
        serialize_with_kind(self, self.serialize_kind(), buf)
    }
}

pub trait XcdrDeserialize: Sized {
    fn deserialize<D: XcdrDeserializer + ?Sized>(deserializer: &mut D) -> XcdrResult<Self>;
}

impl XcdrSerialize for bool {
    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, serializer: &mut S) -> XcdrResult<()> {
        serializer.write_bool(*self)
    }
}

impl XcdrDeserialize for bool {
    fn deserialize<D: XcdrDeserializer + ?Sized>(deserializer: &mut D) -> XcdrResult<Self> {
        deserializer.read_bool()
    }
}

macro_rules! impl_xcdr_for_int {
    ($ty:ty, $write:ident, $read:ident) => {
        impl XcdrSerialize for $ty {
            fn serialize_with<S: XcdrSerializer + ?Sized>(
                &self,
                serializer: &mut S,
            ) -> XcdrResult<()> {
                serializer.$write(*self)
            }
        }

        impl XcdrDeserialize for $ty {
            fn deserialize<D: XcdrDeserializer + ?Sized>(deserializer: &mut D) -> XcdrResult<Self> {
                deserializer.$read()
            }
        }
    };
}

impl_xcdr_for_int!(u8, write_u8, read_u8);
impl_xcdr_for_int!(i8, write_i8, read_i8);
impl_xcdr_for_int!(u16, write_u16, read_u16_le);
impl_xcdr_for_int!(i16, write_i16, read_i16_le);
impl_xcdr_for_int!(u32, write_u32, read_u32_le);
impl_xcdr_for_int!(i32, write_i32, read_i32_le);
impl_xcdr_for_int!(u64, write_u64, read_u64_le);
impl_xcdr_for_int!(i64, write_i64, read_i64_le);

impl XcdrSerialize for f32 {
    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, serializer: &mut S) -> XcdrResult<()> {
        serializer.write_f32(*self)
    }
}

impl XcdrDeserialize for f32 {
    fn deserialize<D: XcdrDeserializer + ?Sized>(deserializer: &mut D) -> XcdrResult<Self> {
        deserializer.read_f32_le()
    }
}

impl XcdrSerialize for f64 {
    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, serializer: &mut S) -> XcdrResult<()> {
        serializer.write_f64(*self)
    }
}

impl XcdrDeserialize for f64 {
    fn deserialize<D: XcdrDeserializer + ?Sized>(deserializer: &mut D) -> XcdrResult<Self> {
        deserializer.read_f64_le()
    }
}

impl XcdrSerialize for char {
    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, serializer: &mut S) -> XcdrResult<()> {
        serializer.write_u32(*self as u32)
    }
}

impl XcdrDeserialize for char {
    fn deserialize<D: XcdrDeserializer + ?Sized>(deserializer: &mut D) -> XcdrResult<Self> {
        let value = deserializer.read_u32_le()?;
        char::from_u32(value).ok_or_else(|| XcdrError::Message("invalid char value".to_string()))
    }
}

impl<T: XcdrSerialize> XcdrSerialize for Box<T> {
    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, serializer: &mut S) -> XcdrResult<()> {
        self.as_ref().serialize_with(serializer)
    }
}

impl<T: XcdrDeserialize> XcdrDeserialize for Box<T> {
    fn deserialize<D: XcdrDeserializer + ?Sized>(deserializer: &mut D) -> XcdrResult<Self> {
        Ok(Box::new(T::deserialize(deserializer)?))
    }
}

impl<T: XcdrSerialize, const N: usize> XcdrSerialize for [T; N] {
    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, serializer: &mut S) -> XcdrResult<()> {
        for item in self {
            item.serialize_with(serializer)?;
        }
        Ok(())
    }
}

impl<T: XcdrDeserialize, const N: usize> XcdrDeserialize for [T; N] {
    fn deserialize<D: XcdrDeserializer + ?Sized>(deserializer: &mut D) -> XcdrResult<Self> {
        let mut data = std::mem::MaybeUninit::<[T; N]>::uninit();
        let ptr = data.as_mut_ptr() as *mut T;
        for idx in 0..N {
            unsafe {
                ptr.add(idx).write(T::deserialize(deserializer)?);
            }
        }
        Ok(unsafe { data.assume_init() })
    }
}

impl XcdrSerialize for String {
    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, _serializer: &mut S) -> XcdrResult<()> {
        Err(XcdrError::Message(
            "String serialization is not supported yet".to_string(),
        ))
    }
}

impl XcdrDeserialize for String {
    fn deserialize<D: XcdrDeserializer + ?Sized>(_deserializer: &mut D) -> XcdrResult<Self> {
        Err(XcdrError::Message(
            "String deserialization is not supported yet".to_string(),
        ))
    }
}

impl<T> XcdrSerialize for Vec<T> {
    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, _serializer: &mut S) -> XcdrResult<()> {
        Err(XcdrError::Message(
            "Vec serialization is not supported yet".to_string(),
        ))
    }
}

impl<T> XcdrDeserialize for Vec<T> {
    fn deserialize<D: XcdrDeserializer + ?Sized>(_deserializer: &mut D) -> XcdrResult<Self> {
        Err(XcdrError::Message(
            "Vec deserialization is not supported yet".to_string(),
        ))
    }
}

impl<K, V> XcdrSerialize for BTreeMap<K, V> {
    fn serialize_with<S: XcdrSerializer + ?Sized>(&self, _serializer: &mut S) -> XcdrResult<()> {
        Err(XcdrError::Message(
            "BTreeMap serialization is not supported yet".to_string(),
        ))
    }
}

fn serialize_with_kind<T: XcdrSerialize + ?Sized>(
    value: &T,
    kind: SerializeKind,
    buf: &mut [u8],
) -> XcdrResult<usize> {
    match kind {
        SerializeKind::Cdr => {
            let mut serializer = cdr::CdrSerialize::new(buf.as_mut_ptr(), buf.len());
            value.serialize_with(&mut serializer)?;
            Ok(serializer.pos)
        }
        SerializeKind::PlainCdr => {
            let mut serializer = plain_cdr::PlainCdrSerialize::new(buf.as_mut_ptr(), buf.len());
            value.serialize_with(&mut serializer)?;
            Ok(serializer.pos)
        }
        SerializeKind::PlCdr => {
            let mut serializer = plcdr::PlcdrSerialize::new(buf.as_mut_ptr(), buf.len());
            value.serialize_with(&mut serializer)?;
            Ok(serializer.pos)
        }
        SerializeKind::PlainCdr2 => {
            let mut serializer = plain_cdr2::PlainCdr2Serialize::new(buf.as_mut_ptr(), buf.len());
            value.serialize_with(&mut serializer)?;
            Ok(serializer.pos)
        }
        SerializeKind::DelimitedCdr => {
            let mut serializer =
                delimited_cdr::DelimitedCdrSerialize::new(buf.as_mut_ptr(), buf.len());
            value.serialize_with(&mut serializer)?;
            Ok(serializer.pos)
        }
        SerializeKind::PlCdr2 => {
            let mut serializer = plcdr2::Plcdr2Serialize::new(buf.as_mut_ptr(), buf.len());
            value.serialize_with(&mut serializer)?;
            Ok(serializer.pos)
        }
    }
}

impl<K, V> XcdrDeserialize for BTreeMap<K, V> {
    fn deserialize<D: XcdrDeserializer + ?Sized>(_deserializer: &mut D) -> XcdrResult<Self> {
        Err(XcdrError::Message(
            "BTreeMap deserialization is not supported yet".to_string(),
        ))
    }
}
