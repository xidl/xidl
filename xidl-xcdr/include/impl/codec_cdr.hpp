#pragma once

#include <cstddef>
#include <cstdint>

extern "C" {
#include "../xidl_xcdr.h"
}

namespace xidl {
namespace xcdr {

struct CdrCodec {
  typedef FfiCdrSerializer Serializer;
  typedef FfiCdrDeserializer Deserializer;

  static Serializer serializer_new(uint8_t *ptr, std::size_t len) {
    return cdr_serializer_new(ptr, len);
  }
  static std::size_t serializer_position(const Serializer *ser) {
    return cdr_serializer_position(ser);
  }
  static void serializer_reset(Serializer *ser) { cdr_serializer_reset(ser); }
  static XcdrFfiError write_u8(Serializer *ser, uint8_t v) {
    return cdr_serializer_write_u8(ser, v);
  }
  static XcdrFfiError write_i8(Serializer *ser, int8_t v) {
    return cdr_serializer_write_i8(ser, v);
  }
  static XcdrFfiError write_bool(Serializer *ser, bool v) {
    return cdr_serializer_write_bool(ser, v);
  }
  static XcdrFfiError write_u16(Serializer *ser, uint16_t v) {
    return cdr_serializer_write_u16(ser, v);
  }
  static XcdrFfiError write_i16(Serializer *ser, int16_t v) {
    return cdr_serializer_write_i16(ser, v);
  }
  static XcdrFfiError write_u32(Serializer *ser, uint32_t v) {
    return cdr_serializer_write_u32(ser, v);
  }
  static XcdrFfiError write_i32(Serializer *ser, int32_t v) {
    return cdr_serializer_write_i32(ser, v);
  }
  static XcdrFfiError write_u64(Serializer *ser, uint64_t v) {
    return cdr_serializer_write_u64(ser, v);
  }
  static XcdrFfiError write_i64(Serializer *ser, int64_t v) {
    return cdr_serializer_write_i64(ser, v);
  }
  static XcdrFfiError write_f32(Serializer *ser, float v) {
    return cdr_serializer_write_f32(ser, v);
  }
  static XcdrFfiError write_f64(Serializer *ser, double v) {
    return cdr_serializer_write_f64(ser, v);
  }
  static XcdrFfiError write_bytes(Serializer *ser, const uint8_t *ptr,
                                  std::size_t len) {
    return cdr_serializer_write_bytes(ser, ptr, len);
  }

  static Deserializer deserializer_new(const uint8_t *ptr, std::size_t len) {
    return cdr_deserializer_new(ptr, len);
  }
  static std::size_t deserializer_position(const Deserializer *de) {
    return cdr_deserializer_position(de);
  }
  static void deserializer_reset(Deserializer *de) {
    cdr_deserializer_reset(de);
  }
  static XcdrFfiError read_u8(Deserializer *de, uint8_t *v) {
    return cdr_deserializer_read_u8(de, v);
  }
  static XcdrFfiError read_i8(Deserializer *de, int8_t *v) {
    return cdr_deserializer_read_i8(de, v);
  }
  static XcdrFfiError read_bool(Deserializer *de, bool *v) {
    return cdr_deserializer_read_bool(de, v);
  }
  static XcdrFfiError read_u16(Deserializer *de, uint16_t *v) {
    return cdr_deserializer_read_u16_le(de, v);
  }
  static XcdrFfiError read_i16(Deserializer *de, int16_t *v) {
    return cdr_deserializer_read_i16_le(de, v);
  }
  static XcdrFfiError read_u32(Deserializer *de, uint32_t *v) {
    return cdr_deserializer_read_u32_le(de, v);
  }
  static XcdrFfiError read_i32(Deserializer *de, int32_t *v) {
    return cdr_deserializer_read_i32_le(de, v);
  }
  static XcdrFfiError read_u64(Deserializer *de, uint64_t *v) {
    return cdr_deserializer_read_u64_le(de, v);
  }
  static XcdrFfiError read_i64(Deserializer *de, int64_t *v) {
    return cdr_deserializer_read_i64_le(de, v);
  }
  static XcdrFfiError read_f32(Deserializer *de, float *v) {
    return cdr_deserializer_read_f32_le(de, v);
  }
  static XcdrFfiError read_f64(Deserializer *de, double *v) {
    return cdr_deserializer_read_f64_le(de, v);
  }
  static XcdrFfiError read_bytes(Deserializer *de, uint8_t *ptr,
                                 std::size_t len) {
    return cdr_deserializer_read_bytes(de, ptr, len);
  }
};

} // namespace xcdr
} // namespace xidl
