#ifndef XIDL_XCDR_H
#define XIDL_XCDR_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum XcdrFfiError {
  Ok = 0,
  BufferOverflow = 1,
  Message = 2,
  NullPointer = 3,
} XcdrFfiError;

typedef struct FfiCdrDeserializer {
  const uint8_t *buf_ptr;
  uintptr_t buf_len;
  uintptr_t pos;
} FfiCdrDeserializer;

typedef struct CdrSerialize {
  uint8_t *buf;
  uintptr_t len;
  uintptr_t pos;
  bool do_io;
} CdrSerialize;

typedef struct CdrSerialize FfiCdrSerializer;

typedef struct FfiPlcdrDeserializer {
  const uint8_t *buf_ptr;
  uintptr_t buf_len;
  uintptr_t pos;
} FfiPlcdrDeserializer;

typedef struct PlcdrSerialize {
  uint8_t *buf;
  uintptr_t len;
  uintptr_t pos;
  bool do_io;
} PlcdrSerialize;

typedef struct PlcdrSerialize FfiPlcdrSerializer;

typedef struct XcdrPlcdrSerialize {
  uint8_t *buf;
  uintptr_t len;
  uintptr_t pos;
  bool do_io;
  uintptr_t field_len_pos;
  uintptr_t field_start_pos;
  bool field_open;
} XcdrPlcdrSerialize;

typedef struct XcdrPlcdrSerialize FfiXcdrPlcdrSerializer;

typedef struct FfiXcdrPlcdrDeserializer {
  const uint8_t *buf_ptr;
  uintptr_t buf_len;
  uintptr_t pos;
  uintptr_t field_end;
  bool field_end_valid;
  bool expecting_len;
} FfiXcdrPlcdrDeserializer;

struct FfiCdrDeserializer cdr_deserializer_new(const uint8_t *buf_ptr, uintptr_t buf_len);

uintptr_t cdr_deserializer_position(const struct FfiCdrDeserializer *self);

void cdr_deserializer_reset(struct FfiCdrDeserializer *self);

enum XcdrFfiError cdr_deserializer_read_u8(struct FfiCdrDeserializer *self, uint8_t *out);

enum XcdrFfiError cdr_deserializer_read_i8(struct FfiCdrDeserializer *self, int8_t *out);

enum XcdrFfiError cdr_deserializer_read_bool(struct FfiCdrDeserializer *self, bool *out);

enum XcdrFfiError cdr_deserializer_read_u16_le(struct FfiCdrDeserializer *self, uint16_t *out);

enum XcdrFfiError cdr_deserializer_read_u16_be(struct FfiCdrDeserializer *self, uint16_t *out);

enum XcdrFfiError cdr_deserializer_read_i16_le(struct FfiCdrDeserializer *self, int16_t *out);

enum XcdrFfiError cdr_deserializer_read_i16_be(struct FfiCdrDeserializer *self, int16_t *out);

enum XcdrFfiError cdr_deserializer_read_u32_le(struct FfiCdrDeserializer *self, uint32_t *out);

enum XcdrFfiError cdr_deserializer_read_u32_be(struct FfiCdrDeserializer *self, uint32_t *out);

enum XcdrFfiError cdr_deserializer_read_i32_le(struct FfiCdrDeserializer *self, int32_t *out);

enum XcdrFfiError cdr_deserializer_read_i32_be(struct FfiCdrDeserializer *self, int32_t *out);

enum XcdrFfiError cdr_deserializer_read_u64_le(struct FfiCdrDeserializer *self, uint64_t *out);

enum XcdrFfiError cdr_deserializer_read_u64_be(struct FfiCdrDeserializer *self, uint64_t *out);

enum XcdrFfiError cdr_deserializer_read_i64_le(struct FfiCdrDeserializer *self, int64_t *out);

enum XcdrFfiError cdr_deserializer_read_i64_be(struct FfiCdrDeserializer *self, int64_t *out);

enum XcdrFfiError cdr_deserializer_read_f32_le(struct FfiCdrDeserializer *self, float *out);

enum XcdrFfiError cdr_deserializer_read_f32_be(struct FfiCdrDeserializer *self, float *out);

enum XcdrFfiError cdr_deserializer_read_f64_le(struct FfiCdrDeserializer *self, double *out);

enum XcdrFfiError cdr_deserializer_read_f64_be(struct FfiCdrDeserializer *self, double *out);

enum XcdrFfiError cdr_deserializer_read_bytes(struct FfiCdrDeserializer *self,
                                              uint8_t *out_ptr,
                                              uintptr_t out_len);

FfiCdrSerializer cdr_serializer_new(uint8_t *buf_ptr, uintptr_t buf_len);

uintptr_t cdr_serializer_position(const FfiCdrSerializer *self);

void cdr_serializer_reset(FfiCdrSerializer *self);

enum XcdrFfiError cdr_serializer_write_u8(FfiCdrSerializer *self, uint8_t val);

enum XcdrFfiError cdr_serializer_write_i8(FfiCdrSerializer *self, int8_t val);

enum XcdrFfiError cdr_serializer_write_bool(FfiCdrSerializer *self, bool val);

enum XcdrFfiError cdr_serializer_write_u16(FfiCdrSerializer *self, uint16_t val);

enum XcdrFfiError cdr_serializer_write_i16(FfiCdrSerializer *self, int16_t val);

enum XcdrFfiError cdr_serializer_write_u32(FfiCdrSerializer *self, uint32_t val);

enum XcdrFfiError cdr_serializer_write_i32(FfiCdrSerializer *self, int32_t val);

enum XcdrFfiError cdr_serializer_write_u64(FfiCdrSerializer *self, uint64_t val);

enum XcdrFfiError cdr_serializer_write_i64(FfiCdrSerializer *self, int64_t val);

enum XcdrFfiError cdr_serializer_write_f32(FfiCdrSerializer *self, float val);

enum XcdrFfiError cdr_serializer_write_f64(FfiCdrSerializer *self, double val);

enum XcdrFfiError cdr_serializer_write_bytes(FfiCdrSerializer *self,
                                             const uint8_t *buf_ptr,
                                             uintptr_t buf_len);

struct FfiPlcdrDeserializer plcdr_deserializer_new(const uint8_t *buf_ptr, uintptr_t buf_len);

uintptr_t plcdr_deserializer_position(const struct FfiPlcdrDeserializer *self);

void plcdr_deserializer_reset(struct FfiPlcdrDeserializer *self);

enum XcdrFfiError plcdr_deserializer_read_u8(struct FfiPlcdrDeserializer *self, uint8_t *out);

enum XcdrFfiError plcdr_deserializer_read_i8(struct FfiPlcdrDeserializer *self, int8_t *out);

enum XcdrFfiError plcdr_deserializer_read_bool(struct FfiPlcdrDeserializer *self, bool *out);

enum XcdrFfiError plcdr_deserializer_read_u16_le(struct FfiPlcdrDeserializer *self, uint16_t *out);

enum XcdrFfiError plcdr_deserializer_read_u16_be(struct FfiPlcdrDeserializer *self, uint16_t *out);

enum XcdrFfiError plcdr_deserializer_read_i16_le(struct FfiPlcdrDeserializer *self, int16_t *out);

enum XcdrFfiError plcdr_deserializer_read_i16_be(struct FfiPlcdrDeserializer *self, int16_t *out);

enum XcdrFfiError plcdr_deserializer_read_u32_le(struct FfiPlcdrDeserializer *self, uint32_t *out);

enum XcdrFfiError plcdr_deserializer_read_u32_be(struct FfiPlcdrDeserializer *self, uint32_t *out);

enum XcdrFfiError plcdr_deserializer_read_i32_le(struct FfiPlcdrDeserializer *self, int32_t *out);

enum XcdrFfiError plcdr_deserializer_read_i32_be(struct FfiPlcdrDeserializer *self, int32_t *out);

enum XcdrFfiError plcdr_deserializer_read_u64_le(struct FfiPlcdrDeserializer *self, uint64_t *out);

enum XcdrFfiError plcdr_deserializer_read_u64_be(struct FfiPlcdrDeserializer *self, uint64_t *out);

enum XcdrFfiError plcdr_deserializer_read_i64_le(struct FfiPlcdrDeserializer *self, int64_t *out);

enum XcdrFfiError plcdr_deserializer_read_i64_be(struct FfiPlcdrDeserializer *self, int64_t *out);

enum XcdrFfiError plcdr_deserializer_read_f32_le(struct FfiPlcdrDeserializer *self, float *out);

enum XcdrFfiError plcdr_deserializer_read_f32_be(struct FfiPlcdrDeserializer *self, float *out);

enum XcdrFfiError plcdr_deserializer_read_f64_le(struct FfiPlcdrDeserializer *self, double *out);

enum XcdrFfiError plcdr_deserializer_read_f64_be(struct FfiPlcdrDeserializer *self, double *out);

enum XcdrFfiError plcdr_deserializer_read_bytes(struct FfiPlcdrDeserializer *self,
                                                uint8_t *out_ptr,
                                                uintptr_t out_len);

FfiPlcdrSerializer plcdr_serializer_new(uint8_t *buf_ptr, uintptr_t buf_len);

uintptr_t plcdr_serializer_position(const FfiPlcdrSerializer *self);

void plcdr_serializer_reset(FfiPlcdrSerializer *self);

enum XcdrFfiError plcdr_serializer_write_u8(FfiPlcdrSerializer *self, uint8_t val);

enum XcdrFfiError plcdr_serializer_write_i8(FfiPlcdrSerializer *self, int8_t val);

enum XcdrFfiError plcdr_serializer_write_bool(FfiPlcdrSerializer *self, bool val);

enum XcdrFfiError plcdr_serializer_write_u16(FfiPlcdrSerializer *self, uint16_t val);

enum XcdrFfiError plcdr_serializer_write_i16(FfiPlcdrSerializer *self, int16_t val);

enum XcdrFfiError plcdr_serializer_write_u32(FfiPlcdrSerializer *self, uint32_t val);

enum XcdrFfiError plcdr_serializer_write_i32(FfiPlcdrSerializer *self, int32_t val);

enum XcdrFfiError plcdr_serializer_write_u64(FfiPlcdrSerializer *self, uint64_t val);

enum XcdrFfiError plcdr_serializer_write_i64(FfiPlcdrSerializer *self, int64_t val);

enum XcdrFfiError plcdr_serializer_write_f32(FfiPlcdrSerializer *self, float val);

enum XcdrFfiError plcdr_serializer_write_f64(FfiPlcdrSerializer *self, double val);

enum XcdrFfiError plcdr_serializer_write_bytes(FfiPlcdrSerializer *self,
                                               const uint8_t *buf_ptr,
                                               uintptr_t buf_len);

FfiXcdrPlcdrSerializer xcdr_plcdr_serializer_new(uint8_t *buf_ptr, uintptr_t buf_len);

uintptr_t xcdr_plcdr_serializer_position(const FfiXcdrPlcdrSerializer *self);

void xcdr_plcdr_serializer_reset(FfiXcdrPlcdrSerializer *self);

enum XcdrFfiError xcdr_plcdr_serializer_begin_field(FfiXcdrPlcdrSerializer *self, uint16_t pid);

enum XcdrFfiError xcdr_plcdr_serializer_end_field(FfiXcdrPlcdrSerializer *self);

enum XcdrFfiError xcdr_plcdr_serializer_write_u8(FfiXcdrPlcdrSerializer *self, uint8_t val);

enum XcdrFfiError xcdr_plcdr_serializer_write_i8(FfiXcdrPlcdrSerializer *self, int8_t val);

enum XcdrFfiError xcdr_plcdr_serializer_write_bool(FfiXcdrPlcdrSerializer *self, bool val);

enum XcdrFfiError xcdr_plcdr_serializer_write_u16(FfiXcdrPlcdrSerializer *self, uint16_t val);

enum XcdrFfiError xcdr_plcdr_serializer_write_i16(FfiXcdrPlcdrSerializer *self, int16_t val);

enum XcdrFfiError xcdr_plcdr_serializer_write_u32(FfiXcdrPlcdrSerializer *self, uint32_t val);

enum XcdrFfiError xcdr_plcdr_serializer_write_i32(FfiXcdrPlcdrSerializer *self, int32_t val);

enum XcdrFfiError xcdr_plcdr_serializer_write_u64(FfiXcdrPlcdrSerializer *self, uint64_t val);

enum XcdrFfiError xcdr_plcdr_serializer_write_i64(FfiXcdrPlcdrSerializer *self, int64_t val);

enum XcdrFfiError xcdr_plcdr_serializer_write_f32(FfiXcdrPlcdrSerializer *self, float val);

enum XcdrFfiError xcdr_plcdr_serializer_write_f64(FfiXcdrPlcdrSerializer *self, double val);

enum XcdrFfiError xcdr_plcdr_serializer_write_bytes(FfiXcdrPlcdrSerializer *self,
                                                    const uint8_t *buf_ptr,
                                                    uintptr_t buf_len);

struct FfiXcdrPlcdrDeserializer xcdr_plcdr_deserializer_new(const uint8_t *buf_ptr,
                                                            uintptr_t buf_len);

uintptr_t xcdr_plcdr_deserializer_position(const struct FfiXcdrPlcdrDeserializer *self);

void xcdr_plcdr_deserializer_reset(struct FfiXcdrPlcdrDeserializer *self);

enum XcdrFfiError xcdr_plcdr_deserializer_next_field(struct FfiXcdrPlcdrDeserializer *self,
                                                     uint16_t *out_pid,
                                                     bool *out_has_field);

enum XcdrFfiError xcdr_plcdr_deserializer_skip_field(struct FfiXcdrPlcdrDeserializer *self);

enum XcdrFfiError xcdr_plcdr_deserializer_read_u8(struct FfiXcdrPlcdrDeserializer *self,
                                                  uint8_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_i8(struct FfiXcdrPlcdrDeserializer *self,
                                                  int8_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_bool(struct FfiXcdrPlcdrDeserializer *self,
                                                    bool *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_u16_le(struct FfiXcdrPlcdrDeserializer *self,
                                                      uint16_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_u16_be(struct FfiXcdrPlcdrDeserializer *self,
                                                      uint16_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_i16_le(struct FfiXcdrPlcdrDeserializer *self,
                                                      int16_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_i16_be(struct FfiXcdrPlcdrDeserializer *self,
                                                      int16_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_u32_le(struct FfiXcdrPlcdrDeserializer *self,
                                                      uint32_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_u32_be(struct FfiXcdrPlcdrDeserializer *self,
                                                      uint32_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_i32_le(struct FfiXcdrPlcdrDeserializer *self,
                                                      int32_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_i32_be(struct FfiXcdrPlcdrDeserializer *self,
                                                      int32_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_u64_le(struct FfiXcdrPlcdrDeserializer *self,
                                                      uint64_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_u64_be(struct FfiXcdrPlcdrDeserializer *self,
                                                      uint64_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_i64_le(struct FfiXcdrPlcdrDeserializer *self,
                                                      int64_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_i64_be(struct FfiXcdrPlcdrDeserializer *self,
                                                      int64_t *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_f32_le(struct FfiXcdrPlcdrDeserializer *self,
                                                      float *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_f32_be(struct FfiXcdrPlcdrDeserializer *self,
                                                      float *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_f64_le(struct FfiXcdrPlcdrDeserializer *self,
                                                      double *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_f64_be(struct FfiXcdrPlcdrDeserializer *self,
                                                      double *out);

enum XcdrFfiError xcdr_plcdr_deserializer_read_bytes(struct FfiXcdrPlcdrDeserializer *self,
                                                     uint8_t *out_ptr,
                                                     uintptr_t out_len);

#endif  /* XIDL_XCDR_H */
