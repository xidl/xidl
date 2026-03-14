# IDL4 to Rust mapping

This document describes how xidlc maps IDL types to Rust types. The mapping
reflects the current Rust generator behavior (including JSON-RPC output) and
does not rely on template definitions.

## General

- IDL identifiers map directly to Rust identifiers.
- Rust keywords are escaped using raw identifiers (e.g., `type` becomes
  `r#type`).
- IDL modules map to Rust modules of the same name.
- IDL declarations outside any module map to Rust items at module root.

```idl
module my_math {
}
```

```rust
mod my_math {
}
```

## Constants

- Numeric constants map to Rust numeric constants with the mapped type.
- `string` and `wstring` constants map to `&'static str`.
- Wide string and wide character literals drop the leading `L`.

```idl
module my_math {
    const string my_string = "My String Value";
    const double PI = 3.141592;
}
```

```rust
mod my_math {
    const my_string: &'static str = "My String Value";
    const PI: f64 = 3.141592;
}
```

```idl
const wstring ws = "Hello World";
```

```rust
const ws: &'static str = "Hello World";
```

## Core Data Types

### Integer Types

| IDL Type           | Rust Type |
| ------------------ | --------- |
| short              | i16       |
| unsigned short     | u16       |
| long               | i32       |
| unsigned long      | u32       |
| long long          | i64       |
| unsigned long long | u64       |
| char               | i8        |
| unsigned char      | u8        |
| int8               | i8        |
| uint8              | u8        |
| int16              | i16       |
| uint16             | u16       |
| int32              | i32       |
| uint32             | u32       |
| int64              | i64       |
| uint64             | u64       |

### Floating-Point Types

| IDL Type    | Rust Type |
| ----------- | --------- |
| float       | f64       |
| double      | f64       |
| long double | f64       |

### Character Types

- IDL `char` and `wchar` map to Rust `char` in type specifications.
- IDL integer types `char` and `unsigned char` map to `i8` and `u8` (see table).

### Boolean Type

- IDL `boolean` maps to Rust `bool`. IDL `TRUE`/`FALSE` map to `true`/`false`.

### Octet Type

- IDL `octet` maps to Rust `u8`.

### Any/Object/ValueBase

- IDL `any`, `object`, and `valuebase` map to raw pointers: `*mut c_void`.

### Native Types

- IDL `native` declarations map to `*mut c_void`.

## Template Types

### Sequences

- `sequence<T>` maps to `Vec<T>`. Bounds are currently ignored.

```idl
typedef sequence<long> V1;
typedef sequence<long, 3> V2;
```

```rust
type V1 = Vec<i32>;
type V2 = Vec<i32>;
```

### Maps

- `map<K, V>` maps to `BTreeMap<K, V>`. Bounds are currently ignored.

```idl
typedef map<string, long> M;
typedef map<string, long, 10> M2;
```

```rust
type M = BTreeMap<String, i32>;
type M2 = BTreeMap<String, i32>;
```

### Arrays

- IDL arrays map to Rust fixed-size arrays using declarator dimensions.

```idl
typedef long Matrix[3][2];
```

```rust
type Matrix = [[i32; 2]; 3];
```

### Strings and Wstrings

- `string` and `wstring` map to Rust `String`.
- Bounded strings are currently treated as unbounded `String`.

### Fixed

- IDL `fixed` maps to Rust `f64`.

## Constructed Types

### Structures

- IDL `struct` maps to a Rust `struct` with the same name.
- Field names are mapped using Rust identifier escaping.
- Array declarators inside structs follow the same array mapping rules.
- No inherent methods are generated for structs. Access is via public fields.
- If `@derive(...)` annotations are present, the generated struct includes the
  corresponding `#[derive(...)]` list.
- If `@rust-xxx(...)` annotations are present on a Rust-emitting declaration or
  member, the generated Rust includes the corresponding raw `#[xxx(...)]`
  attribute lines in source order. When no parameters are provided, generation
  emits `#[xxx]`.
- Struct inheritance is represented as a `base: <Parent>` field.

```idl
@rust-serde(rename_all = "camelCase")
struct User {
    @rust-serde(rename = "userId")
    string user_id;
};
```

```rust
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "userId")]
    pub user_id: String,
}
```

```idl
struct MyStruct {
    long a_long;
    short a_short;
};
```

```rust
pub struct MyStruct {
    pub a_long: i32,
    pub a_short: i16,
}
```

```idl
struct ChildStruct : MyStruct {
    float a_float;
};
```

```rust
pub struct ChildStruct {
    pub base: MyStruct,
    pub a_float: f64,
}
```

### Unions

- IDL `union` maps to a Rust `struct` that stores a discriminator tag and a
  union payload. The discriminator type uses the same mapping rules as other
  types.
- The generated struct provides constructors for each case, a tag accessor, and
  unsafe accessors to the active payload. A `Drop` implementation releases the
  active union field.
- When `Serialize`/`Deserialize` are derived, unions are encoded as externally
  tagged variants keyed by the discriminator case.
- `default` case constructors use `SwitchType::default()` for the tag.

Member functions generated per case `case_x`:

- `new_case_x(value: T) -> Self`
- `is_case_x(&self) -> bool`
- `unsafe fn as_case_x(&self) -> &T`
- `unsafe fn as_case_x_mut(&mut self) -> &mut T`
- `unsafe fn into_case_x(self) -> T`

Common member function:

- `tag(&self) -> &DiscriminatorType`

Example shape (simplified):

```rust
pub enum ExampleKind {
    CaseA,
    CaseB,
}

pub struct Example {
    tag: ExampleKind,
    data: ExampleData,
}

union ExampleData {
    case_a: core::mem::ManuallyDrop<i32>,
    case_b: core::mem::ManuallyDrop<String>,
}

impl Example {
    pub fn new_case_a(value: i32) -> Self { /* ... */ }
    pub fn new_case_b(value: String) -> Self { /* ... */ }
    pub fn tag(&self) -> &ExampleKind { /* ... */ }
    pub unsafe fn as_case_a(&self) -> &i32 { /* ... */ }
    pub unsafe fn as_case_b(&self) -> &String { /* ... */ }
}
```

### Enums

- IDL `enum` maps to a Rust `enum` with the same member names.
- Enums always derive `Debug`, `Clone`, `Copy`, `PartialEq`, and `Eq`.
- `@derive(...)` annotations are not currently applied to enums.

```idl
enum Color {
    Red,
    Green,
    Blue
};
```

```rust
pub enum Color {
    Red,
    Green,
    Blue,
}
```

### Bitsets

- IDL `bitset` maps to a Rust struct with fields for each named bitfield.
- Bitfield widths are currently ignored in the Rust type layout.
- Bitfield types map as follows: `bool`, `octet` -> `u8`, signed -> `i32`,
  unsigned -> `u32`.
- No inherent methods are generated; fields are stored in the generated struct.
- If `@derive(...)` annotations are present, the generated struct includes the
  corresponding `#[derive(...)]` list.
- Bitset inheritance is represented as a `base: <Parent>` field.

```idl
bitset Flags {
    bool enabled: 1;
    unsigned int mode: 3;
};
```

```rust
pub struct Flags {
    pub enabled: bool,
    pub mode: u32,
}
```

Required methods:

- No inherent methods are generated for bitsets.
- Optional `@derive(...)` annotations may add derived trait impls.

### Bitmasks

- IDL `bitmask` maps to a Rust struct with a `value: u32` field.
- Flags are generated as `const` values in declaration order. `@position` and
  `@bit_bound` are currently ignored.
- If `@derive(...)` annotations are present, the generated type includes the
  corresponding `#[derive(...)]` list.

```idl
bitmask Perms {
    Read,
    Write,
    Execute
};
```

```rust
pub struct Perms {
    pub value: u32,
}
```

Required methods:

- No inherent methods are generated for bitmasks.
- Optional `@derive(...)` annotations may add derived trait impls.

### Exceptions

- IDL `exception` maps to a Rust `struct` with the same member layout.
- No inherent methods are generated for exceptions. Access is via public fields.

```idl
exception Error {
    long code;
    string message;
};
```

```rust
pub struct Error {
    pub code: i32,
    pub message: String,
}
```

Required methods:

- No inherent methods are generated for exceptions.

## Interfaces

IDL interfaces are rendered into Rust traits with method signatures derived from
operation and attribute declarations.

- `in` parameters are passed by value for value types (integers, floats, char,
  wchar, bool) and by reference for others.
- `out` and `inout` parameters are passed as `&mut T`.
- `raises` clauses on operations are currently ignored by the Rust generator and
  do not change the generated method signatures.
- Attribute accessors are generated as methods. Readonly attributes generate a
  getter only. Read/write attributes generate a getter and a setter named
  `set_<name>`.
- `raises` clauses on attributes are currently ignored by the Rust generator and
  do not change the generated method signatures.
- `readonly` attributes with `raises` are currently skipped in output.

Example:

```idl
interface Calc {
    long add(in long a, in long b);
    void fill(out sequence<long> data);
    readonly attribute long version;
    attribute string name;
};
```

## Derive Annotations

xidlc supports a builtin `@derive(...)` annotation to attach Rust
`#[derive(...)]` attributes to generated types. This annotation is collected
from the IDL element and applied to structs, unions, bitsets, and bitmasks (not
enums).

Examples:

```idl
@derive(Debug, Clone, PartialEq)
struct Foo {
    long value;
};

@derive(Serialize, Deserialize)
union Payload switch (long) {
    case 1: string text;
    case 2: long number;
};
```

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Foo {
    pub value: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Payload {
    // union layout and methods
}
```

## Other Annotations

- Annotations other than `@derive(...)` are currently ignored by the Rust
  generator, including `@optional`, `@default`, `@range`, `@min`, `@max`,
  `@bit_bound`, `@value`, `@position`, and `@external`.

```rust
trait Calc {
    fn add(&self, a: i32, b: i32) -> i32;
    fn fill(&self, data: &mut Vec<i32>) -> ();
    fn version(&self) -> i32;
    fn name(&self) -> &String;
    fn set_name(&self, value: &String) -> ();
}
```
