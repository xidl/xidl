use std::ffi::CStr;
use std::os::raw::c_char;

#[repr(C)]
pub enum XcdrFormat {
    Cdr = 0,
    Plcdr = 1,
    Cdr3 = 2,
}

pub struct XcdrSerializer {
    _private: [u8; 0],
}

pub struct XcdrDeserializer {
    _private: [u8; 0],
}

struct XcdrSerializerInner {
    serializer: Box<dyn crate::SerializeVisitor>,
    no_buffer: bool,
    serialize_size: usize,
}

struct XcdrDeserializerInner {
    deserializer: Box<dyn crate::DeserializeVisitor>,
}

fn map_format(format: XcdrFormat) -> crate::Format {
    match format {
        XcdrFormat::Cdr => crate::Format::Cdr,
        XcdrFormat::Plcdr => crate::Format::Plcdr,
        XcdrFormat::Cdr3 => crate::Format::Cdr3,
    }
}

unsafe fn serializer_mut<'a>(ptr: *mut XcdrSerializer) -> Option<&'a mut XcdrSerializerInner> {
    if ptr.is_null() {
        return None;
    }
    unsafe { Some(&mut *(ptr as *mut XcdrSerializerInner)) }
}

unsafe fn deserializer_mut<'a>(
    ptr: *mut XcdrDeserializer,
) -> Option<&'a mut XcdrDeserializerInner> {
    if ptr.is_null() {
        return None;
    }
    unsafe { Some(&mut *(ptr as *mut XcdrDeserializerInner)) }
}

fn name_from_c(name: *const c_char) -> Option<String> {
    if name.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(name).to_str().ok().map(|s| s.to_string()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serializer_create(
    format: XcdrFormat,
    no_buffer: bool,
) -> *mut XcdrSerializer {
    let format = map_format(format);
    let inner = XcdrSerializerInner {
        serializer: crate::new_serializer(format),
        no_buffer,
        serialize_size: 0,
    };
    Box::into_raw(Box::new(inner)) as *mut XcdrSerializer
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserializer_create(format: XcdrFormat) -> *mut XcdrDeserializer {
    let format = map_format(format);
    let inner = XcdrDeserializerInner {
        deserializer: crate::new_deserializer(format),
    };
    Box::into_raw(Box::new(inner)) as *mut XcdrDeserializer
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serializer_destroy(ptr: *mut XcdrSerializer) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(ptr as *mut XcdrSerializerInner));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserializer_destroy(ptr: *mut XcdrDeserializer) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(ptr as *mut XcdrDeserializerInner));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_u8(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(1);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_u8(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_u8(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_u8(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_i8(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(1);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_i8(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_i8(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_i8(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_u16(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(2);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_u16(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_u16(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_u16(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_i16(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(2);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_i16(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_i16(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_i16(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_u32(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(4);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_u32(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_u32(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_u32(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_i32(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(4);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_i32(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_i32(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_i32(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_u64(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(8);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_u64(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_u64(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_u64(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_i64(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(8);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_i64(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_i64(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_i64(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_bool(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(1);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_bool(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_bool(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_bool(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_f32(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(4);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_f32(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_f32(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_f32(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_f64(ptr: *mut XcdrSerializer, field: *const c_char) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(8);
    if inner.no_buffer {
        return;
    }
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.serializer.serialize_f64(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_parameter_id(ptr: *mut XcdrSerializer, id: u32) {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    inner.serialize_size = inner.serialize_size.saturating_add(4);
    if inner.no_buffer {
        return;
    }
    let _ = inner.serializer.serialize_parameter_id(id);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserialize_f64(ptr: *mut XcdrDeserializer, field: *const c_char) {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return;
    }
    let inner = inner.unwrap();
    let name = name_from_c(field);
    if name.is_none() {
        return;
    }
    let name = name.unwrap();
    let _ = inner.deserializer.deserialize_f64(&name);
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serialize_size(ptr: *mut XcdrSerializer) -> usize {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return 0;
    }
    inner.unwrap().serialize_size
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_serializer_output(
    ptr: *mut XcdrSerializer,
    len_out: *mut usize,
) -> *const u8 {
    let inner = unsafe { serializer_mut(ptr) };
    if inner.is_none() {
        return std::ptr::null();
    }
    let inner = inner.unwrap();
    let output = inner.serializer.output();
    if !len_out.is_null() {
        unsafe {
            *len_out = output.len();
        }
    }
    output.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn xcdr_deserializer_output(
    ptr: *mut XcdrDeserializer,
    len_out: *mut usize,
) -> *const u8 {
    let inner = unsafe { deserializer_mut(ptr) };
    if inner.is_none() {
        return std::ptr::null();
    }
    let inner = inner.unwrap();
    let output = inner.deserializer.output();
    if !len_out.is_null() {
        unsafe {
            *len_out = output.len();
        }
    }
    output.as_ptr()
}
