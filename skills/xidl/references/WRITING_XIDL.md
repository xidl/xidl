# Writing XIDL: Grammar & Modeling

XIDL is based on OMG IDL with extensions for modern API development. Use this
guide to understand how to define data structures and interfaces.

## 1. Project Metadata (Pragmas)

Every IDL file should start with pragmas to define its scope.

```idl
#pragma xidlc package MyProject.Core
#pragma xidlc version v1.2.0
```

## 2. Modules and Namespaces

Group related types into modules.

```idl
module auth {
    struct Session {
        string id;
    };
};
```

## 3. Data Types

### Primitive Types

| XIDL Type             | Description       |
| :-------------------- | :---------------- |
| `boolean`             | `true` or `false` |
| `octet` / `uint8`     | Unsigned 8-bit    |
| `long` / `int32`      | Signed 32-bit     |
| `unsigned long`       | Unsigned 32-bit   |
| `long long` / `int64` | Signed 64-bit     |
| `float` / `double`    | Floating point    |
| `string` / `wstring`  | UTF-8 string      |

### Complex Types

- **Structs**: Key-value data containers.
- **Enums**: Named constants.
- **Unions**: Discriminated unions (tagged).
- **Bitsets**: Named bit-fields.
- **Bitmasks**: Collection of flags.

### Template Types

- `sequence<T>`: A dynamic array of type T.
- `map<K, V>`: A key-value map.

## 4. Core Annotations

### Serialization

- `@rename("name")`: Rename a field for serialization.
- `@rename_all("snake_case")`: Rename all fields in a struct.
- `@optional`: Mark a field as optional (e.g., `Option<T>` in Rust).

Example:

```idl
@rename_all("camelCase")
struct User {
    uint32 id;
    @optional
    string bio;
};
```

### Inheritance

XIDL supports struct inheritance.

```idl
struct Base {
    long id;
};

struct Derived : Base {
    string name;
};
```
