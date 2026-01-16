#pragma once

#ifdef __cplusplus
extern "C" {
#endif
#include "xidl_xcdr.h"
#ifdef __cplusplus
}
#endif

#include <cstddef>
#include <cstdint>

namespace xidl {
namespace xcdr {

struct BufferView {
  uint8_t *ptr;
  std::size_t len;
  std::size_t pos;

  BufferView() : ptr(NULL), len(0), pos(0) {}
  BufferView(uint8_t *p, std::size_t l) : ptr(p), len(l), pos(0) {}
  BufferView(void *p, std::size_t l)
      : ptr(static_cast<uint8_t *>(p)), len(l), pos(0) {}
  explicit BufferView(XcdrBuffer buf) : ptr(buf.ptr), len(buf.len), pos(buf.pos) {}

  XcdrBuffer c() const { return XcdrBuffer{ptr, len, pos}; }
};

struct ConstBufferView {
  const uint8_t *ptr;
  std::size_t len;
  std::size_t pos;

  ConstBufferView() : ptr(NULL), len(0), pos(0) {}
  ConstBufferView(const uint8_t *p, std::size_t l) : ptr(p), len(l), pos(0) {}
  ConstBufferView(const void *p, std::size_t l)
      : ptr(static_cast<const uint8_t *>(p)), len(l), pos(0) {}
  explicit ConstBufferView(XcdrConstBuffer buf)
      : ptr(buf.ptr), len(buf.len), pos(buf.pos) {}

  XcdrConstBuffer c() const { return XcdrConstBuffer{ptr, len, pos}; }
};

struct BufferResult {
  XcdrFfiError err;
  std::size_t used;

  bool ok() const { return err == XcdrFfiError::Ok; }
};

inline BufferResult from_c(XcdrBufferResult res) {
  return BufferResult{res.err, static_cast<std::size_t>(res.used)};
}

}  // namespace xcdr
}  // namespace xidl
