#ifndef XIDL_XCDR_H
#define XIDL_XCDR_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum XcdrFormat {
  Cdr = 0,
  Plcdr = 1,
  Cdr3 = 2,
} XcdrFormat;

typedef struct XcdrDeserializer XcdrDeserializer;

typedef struct XcdrSerializer XcdrSerializer;

struct XcdrSerializer *xcdr_serializer_create(enum XcdrFormat format, bool no_buffer);

struct XcdrDeserializer *xcdr_deserializer_create(enum XcdrFormat format);

void xcdr_serializer_destroy(struct XcdrSerializer *ptr);

void xcdr_deserializer_destroy(struct XcdrDeserializer *ptr);

void xcdr_serialize_u8(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_u8(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_i8(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_i8(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_u16(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_u16(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_i16(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_i16(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_u32(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_u32(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_i32(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_i32(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_u64(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_u64(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_i64(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_i64(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_bool(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_bool(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_f32(struct XcdrSerializer *ptr, const char *field);

void xcdr_deserialize_f32(struct XcdrDeserializer *ptr, const char *field);

void xcdr_serialize_f64(struct XcdrSerializer *ptr, const char *field);

void xcdr_serialize_parameter_id(struct XcdrSerializer *ptr, uint32_t id);

void xcdr_deserialize_f64(struct XcdrDeserializer *ptr, const char *field);

uintptr_t xcdr_serialize_size(struct XcdrSerializer *ptr);

const uint8_t *xcdr_serializer_output(struct XcdrSerializer *ptr, uintptr_t *len_out);

const uint8_t *xcdr_deserializer_output(struct XcdrDeserializer *ptr, uintptr_t *len_out);

#endif  /* XIDL_XCDR_H */
