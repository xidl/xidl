# IDL4 to C mapping

This document describes how xidlc maps IDL types to C. The mapping reflects the
current C generator behavior.

## General

- IDL modules are flattened; there is no C namespace/module emission.
- Scoped names are joined with `_` in C identifiers.
- Declarations outside modules map to C global scope.
- The generated header includes `<stdbool.h>`, `<stdint.h>`, and `<wchar.h>`.

```idl
module my_math {
    struct Point { long x; long y; };
}
```

```c
typedef struct Point {
  int32_t x;
  int32_t y;
} Point;
```

## Constants

- Numeric constants map to `static const` with the mapped C type.
- `string` constants map to `const char *`.
- `wstring` constants map to `const wchar_t *`.

```idl
const long Answer = 42;
const string Greeting = "hi";
```

```c
static const int32_t Answer = 42;
static const char * Greeting = "hi";
```

## Core Data Types

### Integer Types

| IDL Type           | C Type   |
| ------------------ | -------- |
| short              | int16_t  |
| unsigned short     | uint16_t |
| long               | int32_t  |
| unsigned long      | uint32_t |
| long long          | int64_t  |
| unsigned long long | uint64_t |
| char               | int8_t   |
| unsigned char      | uint8_t  |
| int8               | int8_t   |
| uint8              | uint8_t  |
| int16              | int16_t  |
| uint16             | uint16_t |
| int32              | int32_t  |
| uint32             | uint32_t |
| int64              | int64_t  |
| uint64             | uint64_t |

### Floating-Point Types

| IDL Type    | C Type |
| ----------- | ------ |
| float       | double |
| double      | double |
| long double | double |

### Character Types

- IDL `char` maps to `char`.
- IDL `wchar` maps to `wchar_t`.

### Boolean Type

- IDL `boolean` maps to `bool`.

### Octet Type

- IDL `octet` maps to `uint8_t`.

### Any/Object/ValueBase

- IDL `any`, `object`, and `valuebase` map to `void *`.

## Template Types

### Sequences

- `sequence<T>` maps to `void *` (opaque).
- Bounds are ignored in the C generator.

### Maps

- `map<K, V>` maps to `void *` (opaque).
- Bounds are ignored in the C generator.

### Strings and Wstrings

- `string` maps to `char *`.
- `wstring` maps to `wchar_t *`.
- Bounds are ignored in the C generator.

### Fixed

- `fixed` maps to `double`.

## Constructed Types

### Structures

- IDL `struct` maps to a C `struct` with the same name.
- Array declarators become C array dimensions.
- A default constructor helper and init helper are generated:
  `Struct Struct_new(void);` and `void Struct_init(Struct *self);`

```idl
struct MyStruct {
    long a_long;
    short a_short;
};
```

```c
typedef struct MyStruct {
  int32_t a_long;
  int16_t a_short;
} MyStruct;

MyStruct MyStruct_new(void);
void MyStruct_init(MyStruct* self);
```

### Unions

- IDL `union` maps to a C `struct` with:
  - discriminator field `_d`
  - union payload `_u`
- Case members are fields inside the `_u` union.
- Constructors and init helpers are generated.

```idl
union AUnion switch (octet) {
    case 1: long a_long;
    default: octet a_byte_default;
};
```

```c
typedef struct AUnion {
  uint8_t _d;
  union {
    int32_t a_long;
    uint8_t a_byte_default;
  } _u;
} AUnion;
```

### Enums

- IDL `enum` maps to a C `enum` with the same member names.
- Helpers `Enum_new()` and `Enum_init()` are generated.

```idl
enum Color { Red, Green, Blue };
```

```c
typedef enum Color {
  Red,
  Green,
  Blue,
} Color;
```

### Bitsets

- IDL `bitset` maps to a C `struct` with fields for each bitfield.
- Bitfield widths are not represented in the C type layout; widths are shown in
  comments as `/* pos N */`.
- Helpers for set operations are generated (insert/remove/toggle/etc.).

```idl
bitset Flags {
    bool enabled: 1;
    unsigned int mode: 3;
};
```

```c
typedef struct Flags {
  bool enabled; /* pos 1 */
  uint32_t mode; /* pos 3 */
} Flags;
```

### Bitmasks

- IDL `bitmask` maps to a C `enum`.
- Helpers for set operations are generated.

```idl
bitmask Perms { Read, Write, Execute };
```

```c
typedef enum Perms {
  Read,
  Write,
  Execute,
} Perms;
```

## Interfaces

- Interfaces map to a set of C function declarations.
- `out`/`inout` parameters are passed as pointers.
- `in` parameters are passed by value.
- Attributes map to `get_attribute_<name>` and `set_attribute_<name>` functions.
- Readonly attributes with `raises` are currently skipped.

```idl
interface Calc {
    long add(in long a, in long b);
    readonly attribute long version;
    attribute string name;
};
```

```c
/* Interface Calc */
int32_t Calc_add(int32_t a, int32_t b);
int32_t Calc_get_attribute_version(void);
char * Calc_get_attribute_name(void);
void Calc_set_attribute_name(char * name);
```

## Exceptions, Typedefs, Native

- `exception` declarations are ignored by the C generator.
- `typedef` declarations are ignored by the C generator.
- `native` declarations are ignored by the C generator.

## XCDR Support

- For structs, unions, enums, bitsets, and bitmasks, the C generator also emits
  XCDR serialization helpers in separate `*_xcdr.h/.c` outputs.
