#pragma once

#include <cstddef>
#include <cstdint>

extern "C" {
#include "../xidl_xcdr.h"
}

#include "./buffer_view.hpp"
#include "./codec_cdr.hpp"
#include "./codec_delimited_cdr.hpp"
#include "./codec_plain_cdr2.hpp"
#include "./codec_plcdr.hpp"
#include "./codec_plcdr2.hpp"
#include "./codec_xcdr_plcdr.hpp"

namespace xidl {
namespace xcdr {

template <typename Codec> class Serializer {
public:
  typedef typename Codec::Serializer FfiSerializer;

  explicit Serializer(BufferView buf)
      : inner_(Codec::serializer_new(buf.ptr, buf.len)) {}

  std::size_t position() const { return Codec::serializer_position(&inner_); }

  void reset() { Codec::serializer_reset(&inner_); }

  template <typename T> BufferResult write(const T &value) {
    return Traits<T>::template serialize<Codec>(*this, value);
  }

  BufferResult write_u8(uint8_t value) {
    return make_result(Codec::write_u8(&inner_, value));
  }
  BufferResult write_i8(int8_t value) {
    return make_result(Codec::write_i8(&inner_, value));
  }
  BufferResult write_bool(bool value) {
    return make_result(Codec::write_bool(&inner_, value));
  }
  BufferResult write_u16(uint16_t value) {
    return make_result(Codec::write_u16(&inner_, value));
  }
  BufferResult write_i16(int16_t value) {
    return make_result(Codec::write_i16(&inner_, value));
  }
  BufferResult write_u32(uint32_t value) {
    return make_result(Codec::write_u32(&inner_, value));
  }
  BufferResult write_i32(int32_t value) {
    return make_result(Codec::write_i32(&inner_, value));
  }
  BufferResult write_u64(uint64_t value) {
    return make_result(Codec::write_u64(&inner_, value));
  }
  BufferResult write_i64(int64_t value) {
    return make_result(Codec::write_i64(&inner_, value));
  }
  BufferResult write_f32(float value) {
    return make_result(Codec::write_f32(&inner_, value));
  }
  BufferResult write_f64(double value) {
    return make_result(Codec::write_f64(&inner_, value));
  }
  BufferResult write_bytes(const uint8_t *ptr, std::size_t len) {
    return make_result(Codec::write_bytes(&inner_, ptr, len));
  }

private:
  BufferResult make_result(XcdrFfiError err) const {
    return BufferResult{err, position()};
  }

  FfiSerializer inner_;
};

template <typename Codec> class Deserializer {
public:
  typedef typename Codec::Deserializer FfiDeserializer;

  explicit Deserializer(ConstBufferView buf)
      : inner_(Codec::deserializer_new(buf.ptr, buf.len)) {}

  std::size_t position() const { return Codec::deserializer_position(&inner_); }

  void reset() { Codec::deserializer_reset(&inner_); }

  template <typename T> BufferResult read(T &value) {
    return Traits<T>::template deserialize<Codec>(*this, value);
  }

  BufferResult read_u8(uint8_t &value) {
    return make_result(Codec::read_u8(&inner_, &value));
  }
  BufferResult read_i8(int8_t &value) {
    return make_result(Codec::read_i8(&inner_, &value));
  }
  BufferResult read_bool(bool &value) {
    return make_result(Codec::read_bool(&inner_, &value));
  }
  BufferResult read_u16(uint16_t &value) {
    return make_result(Codec::read_u16(&inner_, &value));
  }
  BufferResult read_i16(int16_t &value) {
    return make_result(Codec::read_i16(&inner_, &value));
  }
  BufferResult read_u32(uint32_t &value) {
    return make_result(Codec::read_u32(&inner_, &value));
  }
  BufferResult read_i32(int32_t &value) {
    return make_result(Codec::read_i32(&inner_, &value));
  }
  BufferResult read_u64(uint64_t &value) {
    return make_result(Codec::read_u64(&inner_, &value));
  }
  BufferResult read_i64(int64_t &value) {
    return make_result(Codec::read_i64(&inner_, &value));
  }
  BufferResult read_f32(float &value) {
    return make_result(Codec::read_f32(&inner_, &value));
  }
  BufferResult read_f64(double &value) {
    return make_result(Codec::read_f64(&inner_, &value));
  }
  BufferResult read_bytes(uint8_t *ptr, std::size_t len) {
    return make_result(Codec::read_bytes(&inner_, ptr, len));
  }

private:
  BufferResult make_result(XcdrFfiError err) const {
    return BufferResult{err, position()};
  }

  FfiDeserializer inner_;
};

typedef Serializer<CdrCodec> CdrSerializer;
typedef Deserializer<CdrCodec> CdrDeserializer;
typedef Serializer<DelimitedCdrCodec> DelimitedCdrSerializer;
typedef Deserializer<DelimitedCdrCodec> DelimitedCdrDeserializer;
typedef Serializer<PlainCdr2Codec> PlainCdr2Serializer;
typedef Deserializer<PlainCdr2Codec> PlainCdr2Deserializer;
typedef Serializer<PlcdrCodec> PlcdrSerializer;
typedef Deserializer<PlcdrCodec> PlcdrDeserializer;
typedef Serializer<Plcdr2Codec> Plcdr2Serializer;
typedef Deserializer<Plcdr2Codec> Plcdr2Deserializer;
typedef Serializer<XcdrPlcdrCodec> XcdrPlcdrSerializer;
typedef Deserializer<XcdrPlcdrCodec> XcdrPlcdrDeserializer;

} // namespace xcdr
} // namespace xidl
