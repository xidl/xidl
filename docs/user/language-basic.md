# Language Basics

XIDL is an Interface Definition Language (IDL) based on the standard OMG IDL, extended with modern features such as Annotations and Pragmas. It is primarily used to define cross-language, cross-platform serialization protocols and RPC interfaces.

## Base Types

XIDL supports the following base data types:

| Type                 | Description                     | Notes                                      |
| :---                 | :---                            | :---                                       |
| `boolean`            | Boolean value                   | `true` or `false`                          |
| `int8`               | Signed 8-bit integer            | Equivalent to `int8_t`                     |
| `uint8`              | Unsigned 8-bit integer          | Equivalent to `uint8_t`                    |
| `short`              | Signed 16-bit integer           |                                            |
| `unsigned short`     | Unsigned 16-bit integer         |                                            |
| `long`               | Signed 32-bit integer           |                                            |
| `unsigned long`      | Unsigned 32-bit integer         |                                            |
| `long long`          | Signed 64-bit integer           |                                            |
| `unsigned long long` | Unsigned 64-bit integer         |                                            |
| `float`              | Single-precision floating point |                                            |
| `double`             | Double-precision floating point |                                            |
| `char`               | Single-byte character           |                                            |
| `wchar`              | Wide character                  |                                            |
| `string`             | String                          | Optional length limit, e.g., `string<256>` |
| `wstring`            | Wide string                     | Optional length limit                      |
| `octet`              | 8-bit byte                      | Equivalent to `byte`                       |
| `any`                | Dynamic type                    |                                            |

## Complex Types

### Struct

Structs are used to group multiple fields of different types. Inheritance is supported.

```idl
struct SimpleStruct {
    uint8 a;
};

struct ComplexStruct : SimpleStruct {
    string name;
    sequence<uint8> data;
};
```

### Enum

Enums define a set of named constant values.

```idl
enum Status {
    OK,
    ERROR,
    @default_literal
    UNKNOWN
};
```

### Union

Unions allow storing different types in a single memory location, determined by a discriminant (`switch`).

```idl
union MyUnion switch (long) {
    case 1:
        long long_value;
    case 2:
        string string_value;
    default:
        boolean bool_value;
};
```

### Collections and Maps

- `sequence<T>`: Dynamic array, optional length limit `sequence<T, length>`.
- `map<K, V>`: Key-value map, optional length limit `map<K, V, length>`.

### Bitset and Bitmask

- `bitset`: Allows defining bit-level fields.
- `bitmask`: Similar to enum, but specifically for bitwise operations.

```idl
bitset MyBitset {
    bitfield<1, boolean> flag1;
    bitfield<2, uint8> value;
};

bitmask MyFlags {
    FLAG_A,
    FLAG_B
};
```

### Typedef

`typedef` is used to create an alias for an existing type.

```idl
typedef sequence<uint8> Bytes;
typedef string<128> ShortString;
```

### Constant

Constants define fixed values that can be used throughout the IDL.

```idl
const long MAX_BUFFER_SIZE = 1024;
const string DEFAULT_NAME = "XIDL";
```

## Module

Modules are used to organize code and provide namespace isolation.

```idl
module MyModule {
    struct MyData {
        long id;
    };
};
```

## Interface

Interfaces define a set of operations (methods).

```idl
interface MyService {
    void doSomething(in string input);
    long calculate(in long a, in long b);
};
```

## Annotation

Annotations are used to add metadata to IDL elements, affecting code generation or serialization behavior.

```idl
@post(path = "/hello")
void sayHello(
    @position(0) in string name
);
```

Common built-in annotations:
- `@id(value)`: Specifies the field ID.
- `@optional`: Marks a field as optional.
- `@default(value)`: Specifies a default value.
- `@range(min, max)`: Specifies a range of values.

## Pragma

Preprocessing instructions start with `#pragma` and are used to configure compiler behavior.

```idl
#pragma xidlc service http://127.0.0.1:8080 dev server
```

## Comments

Supports single-line comments `//` and multi-line comments `/* ... */`. Additionally, documentation comments (starting with `/**` or `///`) are supported; these comments are extracted and preserved in the generated code.
