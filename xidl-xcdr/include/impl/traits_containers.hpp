#pragma once

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

#include "./serializer.hpp"

namespace xidl {
namespace xcdr {

template <typename T, std::size_t N> struct Traits<std::array<T, N>> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser,
                                const std::array<T, N> &value) {
    for (std::size_t i = 0; i < N; ++i) {
      BufferResult res = ser.write(value[i]);
      if (!res.ok()) {
        return res;
      }
    }
    return BufferResult{XcdrFfiError::Ok, ser.position()};
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de,
                                  std::array<T, N> &value) {
    for (std::size_t i = 0; i < N; ++i) {
      BufferResult res = de.read(value[i]);
      if (!res.ok()) {
        return res;
      }
    }
    return BufferResult{XcdrFfiError::Ok, de.position()};
  }
};

template <typename T> struct Traits<std::vector<T>> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser,
                                const std::vector<T> &value) {
    if (value.size() > std::numeric_limits<uint32_t>::max()) {
      return BufferResult{XcdrFfiError::Message, ser.position()};
    }
    uint32_t len = static_cast<uint32_t>(value.size());
    BufferResult res = ser.write(len);
    if (!res.ok()) {
      return res;
    }
    for (std::size_t i = 0; i < value.size(); ++i) {
      res = ser.write(value[i]);
      if (!res.ok()) {
        return res;
      }
    }
    return BufferResult{XcdrFfiError::Ok, ser.position()};
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de,
                                  std::vector<T> &value) {
    uint32_t len = 0;
    BufferResult res = de.read(len);
    if (!res.ok()) {
      return res;
    }
    value.clear();
    value.resize(len);
    for (uint32_t i = 0; i < len; ++i) {
      res = de.read(value[i]);
      if (!res.ok()) {
        return res;
      }
    }
    return BufferResult{XcdrFfiError::Ok, de.position()};
  }
};

template <> struct Traits<std::string> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser,
                                const std::string &value) {
    if (value.size() > std::numeric_limits<uint32_t>::max()) {
      return BufferResult{XcdrFfiError::Message, ser.position()};
    }
    uint32_t len = static_cast<uint32_t>(value.size());
    BufferResult res = ser.write(len);
    if (!res.ok()) {
      return res;
    }
    if (len > 0) {
      res =
          ser.write_bytes(reinterpret_cast<const uint8_t *>(value.data()), len);
      if (!res.ok()) {
        return res;
      }
    }
    return BufferResult{XcdrFfiError::Ok, ser.position()};
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de, std::string &value) {
    uint32_t len = 0;
    BufferResult res = de.read(len);
    if (!res.ok()) {
      return res;
    }
    value.clear();
    value.resize(len);
    if (len > 0) {
      res = de.read_bytes(reinterpret_cast<uint8_t *>(&value[0]), len);
      if (!res.ok()) {
        return res;
      }
    }
    return BufferResult{XcdrFfiError::Ok, de.position()};
  }
};

template <> struct Traits<std::wstring> {
  template <typename Codec>
  static BufferResult serialize(Serializer<Codec> &ser,
                                const std::wstring &value) {
    if (value.size() > std::numeric_limits<uint32_t>::max()) {
      return BufferResult{XcdrFfiError::Message, ser.position()};
    }
    uint32_t len = static_cast<uint32_t>(value.size());
    BufferResult res = ser.write(len);
    if (!res.ok()) {
      return res;
    }
    for (std::size_t i = 0; i < value.size(); ++i) {
      res = ser.write(static_cast<wchar_t>(value[i]));
      if (!res.ok()) {
        return res;
      }
    }
    return BufferResult{XcdrFfiError::Ok, ser.position()};
  }

  template <typename Codec>
  static BufferResult deserialize(Deserializer<Codec> &de,
                                  std::wstring &value) {
    uint32_t len = 0;
    BufferResult res = de.read(len);
    if (!res.ok()) {
      return res;
    }
    value.clear();
    value.resize(len);
    for (uint32_t i = 0; i < len; ++i) {
      wchar_t ch = 0;
      res = de.read(ch);
      if (!res.ok()) {
        return res;
      }
      value[i] = ch;
    }
    return BufferResult{XcdrFfiError::Ok, de.position()};
  }
};

} // namespace xcdr
} // namespace xidl
