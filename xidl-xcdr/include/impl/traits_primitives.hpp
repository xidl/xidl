#pragma once

#include <cstddef>
#include <cstdint>

#include "./serializer.hpp"

namespace xidl {
namespace xcdr {

template <> struct Traits<uint8_t> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, uint8_t value) {
    return ser.write_u8(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, uint8_t &value) {
    return de.read_u8(value);
  }
};

template <> struct Traits<int8_t> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, int8_t value) {
    return ser.write_i8(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, int8_t &value) {
    return de.read_i8(value);
  }
};

template <> struct Traits<uint16_t> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, uint16_t value) {
    return ser.write_u16(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, uint16_t &value) {
    return de.read_u16(value);
  }
};

template <> struct Traits<int16_t> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, int16_t value) {
    return ser.write_i16(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, int16_t &value) {
    return de.read_i16(value);
  }
};

template <> struct Traits<uint32_t> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, uint32_t value) {
    return ser.write_u32(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, uint32_t &value) {
    return de.read_u32(value);
  }
};

template <> struct Traits<int32_t> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, int32_t value) {
    return ser.write_i32(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, int32_t &value) {
    return de.read_i32(value);
  }
};

template <> struct Traits<uint64_t> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, uint64_t value) {
    return ser.write_u64(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, uint64_t &value) {
    return de.read_u64(value);
  }
};

template <> struct Traits<int64_t> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, int64_t value) {
    return ser.write_i64(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, int64_t &value) {
    return de.read_i64(value);
  }
};

template <> struct Traits<bool> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, bool value) {
    return ser.write_bool(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, bool &value) {
    return de.read_bool(value);
  }
};

template <> struct Traits<float> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, float value) {
    return ser.write_f32(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, float &value) {
    return de.read_f32(value);
  }
};

template <> struct Traits<double> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, double value) {
    return ser.write_f64(value);
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, double &value) {
    return de.read_f64(value);
  }
};

template <> struct Traits<char> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, char value) {
    return ser.write_i8(static_cast<int8_t>(value));
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, char &value) {
    int8_t tmp = 0;
    BufferResult res = de.read_i8(tmp);
    if (res.ok()) {
      value = static_cast<char>(tmp);
    }
    return res;
  }
};

template <> struct Traits<wchar_t> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser, wchar_t value) {
    return ser.write_u32(static_cast<uint32_t>(value));
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, wchar_t &value) {
    uint32_t tmp = 0;
    BufferResult res = de.read_u32(tmp);
    if (res.ok()) {
      value = static_cast<wchar_t>(tmp);
    }
    return res;
  }
};

} // namespace xcdr
} // namespace xidl
