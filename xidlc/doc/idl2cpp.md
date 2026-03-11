# IDL4 to C++ mapping

This document distills the core rules from `idl4-to-cpp.pdf.md`. It is a
practical summary of how IDL constructs map to C++ types and APIs.

## General

- IDL identifiers map to equivalent C++ identifiers with no change.
- IDL modules map to C++ namespaces with the same name.
- C++11 is the minimum required standard.
- Any C++ keyword used as an IDL name gets a leading `_` in C++.
- IDL declarations outside any module map to C++ global scope.

### IDL Type Traits

Implementations provide the following traits in `omg::types`:

- `value_type<T>`: type used as return C++ type.
- `in_type<T>`: type used as `in` C++ type.
- `out_type<T>`: type used as `out` C++ type.
- `inout_type<T>`: type used as `inout` C++ type.

Each trait has `_t` or `_v` aliases (e.g. `in_type_t<T>`).

Additional traits are specialized per type category:

- Sequences/strings/wstrings/maps: `is_bounded<T>` and `bound<T>`.
- Arrays: `dimensions<T>`.
- Fixed: `digits<T>` and `scale<T>`.
- Enum/bitmask with `@bit_bound`: `bit_bound<T>` and `underlying_type<T>`.
- Maps: `key<T>` and `elements<T>`.

```idl
module my_math {
};
```

```cpp
namespace my_math {
}
```

## Constants

- Numeric and boolean constants map to `constexpr` values of the mapped type.
- `string` constants map to `omg::types::string_view` (or `std::string_view` in
  C++17+ implementations).
- `wstring` constants map to `omg::types::wstring_view` (or `std::wstring_view`
  in C++17+ implementations).
- Wide strings and wide characters use the `L` prefix in C++ literals.

```idl
module my_math {
    const string my_string = "My String Value";
    const double PI = 3.141592;
};
```

```cpp
namespace my_math {
    constexpr omg::types::string_view my_string = "My String Value";
    constexpr double PI = 3.141592;
}
```

```idl
const wstring ws = "Hello World";
```

```cpp
constexpr omg::types::wstring_view ws{ L"Hello World" };
```

## Core Data Types

### Integer Types

| IDL Type           | C++ Type |
| ------------------ | -------- |
| short              | int16_t  |
| unsigned short     | uint16_t |
| long               | int32_t  |
| unsigned long      | uint32_t |
| long long          | int64_t  |
| unsigned long long | uint64_t |

### Floating-Point Types

| IDL Type    | C++ Type    |
| ----------- | ----------- |
| float       | float       |
| double      | double      |
| long double | long double |

### Other Basic Types

- `char` -> `char`
- `wchar` -> `wchar_t`
- `boolean` -> `bool`
- `octet` -> `uint8_t`
- Default values: integers and floats `0`, `bool` is `false`, `char/wchar` is
  `0`.

## Template Types

### Sequences

- `sequence<T>` maps to `std::vector<T>` or `omg::types::sequence<T>`.
- Bounded sequences map to `std::vector<T>` or
  `omg::types::bounded_sequence<T, N>`.
- Implementations must define `omg::types::sequence<T>` and
  `omg::types::bounded_sequence<T, N>` and support conversion to/from
  `std::vector<T>`.
- Bounded sequences require a bounds check (often at serialization time).

```idl
typedef sequence<long> V1;
typedef sequence<long, 3> V2;
```

```cpp
using V1 = omg::types::sequence<int32_t>;
using V2 = omg::types::bounded_sequence<int32_t, 3>;
```

### Strings / Wstrings

- `string` -> `std::string` or `omg::types::string`.
- `wstring` -> `std::wstring` or `omg::types::wstring`.
- Bounded strings use `omg::types::bounded_string<N>` / `bounded_wstring<N>`.
- Implementations must define `omg::types::string`, `bounded_string<N>`,
  `omg::types::wstring`, and `bounded_wstring<N>` and support conversion to/from
  `std::string` / `std::wstring`.
- Bounded strings require a bounds check (often at serialization time).

```idl
typedef string U1;
typedef string<8> U2;
typedef wstring<4> W1;
```

```cpp
using U1 = omg::types::string;
using U2 = omg::types::bounded_string<8>;
using W1 = omg::types::bounded_wstring<4>;
```

### Fixed

- `fixed` maps to a class `omg::types::fixed` with arithmetic, conversion, and
  formatting operations.
- The class is default constructible and constructible from integral and
  floating types.

```idl
typedef fixed<5,2> Price;
```

```cpp
using Price = omg::types::fixed; // or implementation-defined equivalent
```

## Constructed Types

### Structures

An IDL `struct` maps to a C++ `struct` with the same name. Each IDL member is
mapped to a public C++ member of the corresponding mapped type, preserving
member order. Built-in members are value-initialized to their default values,
enumerators to their first value, and other members via their default
constructors.

```idl
struct MyStruct {
    long a_long;
    short a_short;
};
```

```cpp
struct MyStruct {
    int32_t a_long {};
    int16_t a_short {};
};
```

Required member functions for mapped structs:

- Default constructor (initializes members per mapping rules).
- Copy constructor (deep copy).
- Move constructor.
- Copy assignment operator (deep copy with strong safety).
- Move assignment operator.
- Comparison operators (at least `==` and `!=`).
- Destructor (releases members).

Required free function:

- `void swap(MyStruct& a, MyStruct& b);` in the same namespace as `MyStruct`.
- Struct inheritance (single base) maps to public inheritance; `swap` must also
  swap inherited members.

```idl
struct ChildStruct : MyStruct {
    float a_float;
};
```

```cpp
struct ChildStruct : MyStruct {
    float a_float {};
};
```

### Unions

- `union` maps to a C++ class with constructors/assignments like structs.
- `_d()` accessors get/set the discriminator value.
- Each member has accessors and modifiers; non-trivial members use references.
- If the member can be selected by multiple labels, modifiers accept an extra
  discriminator parameter.
- If an implicit default exists, `_default()` is provided.
- A `swap(Union&, Union&)` is provided in the class scope.
- Discriminator types include integral, enum, `char`, `wchar`, `boolean`, and
  (with extended data types) `octet`, `int8`, `uint8`.

```idl
union AUnion switch (octet) {
    case 1: long a_long;
    case 2: case 3: short a_short;
    default: octet a_byte_default;
};
```

```cpp
class AUnion {
public:
    uint8_t _d() const;
    void _d(uint8_t value);
    void a_long(int32_t value);
    int32_t a_long() const;
    void a_short(int16_t value, uint8_t discriminator);
    int16_t a_short() const;
    void a_byte_default(uint8_t value);
    uint8_t a_byte_default() const;
};
```

### Enums

- `enum` maps to a scoped `enum class`.
- If `@bit_bound` is present, the enum underlying type is: `int8_t` (1..8),
  `int16_t` (9..16), `int32_t` (17..32), `int64_t` (33..64).
- If `@value` is present, the enumerator value is set accordingly.

```idl
@bit_bound(6)
enum Color {
    @value(1) red,
    @value(2) green
};
```

```cpp
enum class Color : int8_t {
    red = 1,
    green = 2
};
```

### Arrays

- IDL arrays map to `std::array<T, N>` or `omg::types::array<T, N>`.
- Multidimensional arrays are nested `std::array`/`omg::types::array`.
- Implementations must define `omg::types::array<T, N>` and support conversion
  to/from `std::array`.

```idl
typedef long long_array[100];
typedef string string_array[1][2];
```

```cpp
using long_array = omg::types::array<int32_t, 100>;
using string_array = omg::types::array<omg::types::array<omg::types::string, 2>, 1>;
```

### Typedefs

- `typedef` maps to a `using` alias.

## Any

- `any` maps to `omg::types::Any` (platform-specific implementation).

## Interfaces

- An IDL interface maps to a C++ class; inheritance follows the IDL base list.
- Attributes map to pure virtual getter/setter pairs. Readonly attributes omit
  the setter.
- Operations map to pure virtual methods. `out`/`inout` are passed by reference.
- `in` parameters are passed by value for built-in types and enums, otherwise by
  `const&`.
- Parameters of interface type `T` map to `std::shared_ptr<T>` or
  `omg::types::ref_type<T>`. Implementations also provide
  `omg::types::weak_ref_type<T>` (like `std::weak_ptr`).

```idl
interface AnInterface {
    attribute long attr;
    readonly attribute long ro_attr;
    void op1(in long i_param, in MyStruct si_param, inout long io_param);
}
```

```cpp
class AnInterface {
public:
    virtual void attr(int32_t value) = 0;
    virtual int32_t attr() const = 0;
    virtual int32_t ro_attr() const = 0;
    virtual void op1(int32_t i_param, const MyStruct& si_param, int32_t& io_param) = 0;
};
```

## Native Types

- `native` types are implementation-defined; the spec allows mappings but does
  not define any concrete native types.

## Exceptions

- IDL exceptions map to C++ classes derived from `std::exception`.
- Default ctor initializes members to defaults.
- Copy/move ctor, copy/move assignment, destructor are required.
- Provide `what()` and a convenience ctor with parameters for members plus a
  `const char*` message.

```idl
exception AnException { long error_code; };
```

```cpp
class AnException : public std::exception {
public:
    AnException(int32_t error_code, const char* what);
    int32_t error_code() const;
};
```

## Interface Forward Declarations

- `interface Foo;` maps to `class Foo;`.

## Interfaces - Full

- Types, exceptions, and constants declared inside an interface map to nested
  C++ declarations. Constants become `static constexpr`.

```idl
interface FullInterface {
    struct S { long a_long; };
    const double PI = 3.14;
    void op1(in S s_in);
};
```

```cpp
class FullInterface {
public:
    struct S { int32_t a_long; };
    static constexpr double PI = 3.14;
    virtual void op1(const S& s_in) = 0;
};
```

## Value Types

- `valuetype` maps to a C++ class.
- Public state -> public pure virtual accessors/modifiers.
- Private state -> protected pure virtual accessors/modifiers.
- Factory operations generate `<ValueType>_factory` with pure virtual factory
  methods returning `std::shared_ptr<T>` or `omg::types::ref_type<T>`.
- If a valuetype supports an interface, its operations are pure virtual methods
  on the mapped class.

```idl
valuetype VT1 {
    attribute long a_long_attr;
    public long a_public_long;
    private long a_private_long;
    factory vt_factory(in long a_long, in short a_short);
};
```

```cpp
class VT1 {
public:
    virtual void a_long_attr(int32_t value) = 0;
    virtual int32_t a_long_attr() const = 0;
    virtual void a_public_long(int32_t value) = 0;
protected:
    virtual void a_private_long(int32_t value) = 0;
};

class VT1_factory {
public:
    virtual omg::types::ref_type<VT1> vt_factory(int32_t a_long, int16_t a_short) = 0;
protected:
    ~VT1_factory() = default;
};
```

## Extended Data Types

### Maps

- `map<K, V>` maps to `std::map<K, V>` or `omg::types::map<K, V>`.
- `map<K, V, N>` maps to `omg::types::bounded_map<K, V, N>` or `std::map`.
- Implementations must define `omg::types::map` and `bounded_map` and support
  conversion to/from `std::map`. Bounded maps require bounds checks.

```idl
typedef map<unsigned long, MyStruct> M1;
typedef map<string, MyStruct, 20> M2;
```

```cpp
using M1 = omg::types::map<uint32_t, MyStruct>;
using M2 = omg::types::bounded_map<omg::types::string, MyStruct, 20>;
```

### Bitsets

- `bitset` maps to a C++ `struct` with bitfield members and aggregate init.
- Inheritance maps to public inheritance.

```idl
bitset BitSet1 {
    bitfield<1> bit0;
    bitfield<2, unsigned short> bits2_3;
};
```

```cpp
struct BitSet1 {
    bool bit0 : 1;
    uint16_t bits2_3 : 2;
};
```

### Bitmasks

- `bitmask` maps to a C++ `struct` with an unscoped enum `_flags` whose
  underlying type is the smallest unsigned integer fitting `@bit_bound`.
- Stores a `_value` member of that underlying type and provides bitwise ops.

```idl
@bit_bound(16)
bitmask MyBitMask {
    @position(0) flag0,
    @position(1) flag1
};
```

```cpp
struct MyBitMask {
    enum _flags : uint16_t {
        flag0 = 0x01 << 0,
        flag1 = 0x01 << 1
    };
};
```

### 8-bit and Explicitly-Named Integers

| IDL Type | C++ Type |
| -------- | -------- |
| int8     | int8_t   |
| uint8    | uint8_t  |
| int16    | int16_t  |
| uint16   | uint16_t |
| int32    | int32_t  |
| uint32   | uint32_t |
| int64    | int64_t  |
| uint64   | uint64_t |

## Anonymous Types

- Anonymous types do not change the mapping rules. Use `decltype` in C++ if you
  need the declared type.

## Annotations Impacting Mapping

- `@optional`: maps to `std::optional<T>` (C++17+) or `omg::types::optional<T>`.
- `@default_literal`: initialize to the annotated literal.
- `@default`: initialize to the annotated value.
- `@range(min,max)`: maps to `omg::types::ranged<T, min, max>` and throws
  `std::out_of_range` on invalid assignment.
- `@min` / `@max`: setters enforce range and throw `std::out_of_range`.
- `@bit_bound`: affects enum underlying type and bitmask mapping.
- `@external`: maps to `std::shared_ptr<T>` or `omg::types::ref_type<T>` and
  struct copy ctor performs a deep copy of the external member.
- `@value`: sets enum enumerator values.
- `@position`: affects bitmask enumerator values.
- `@verbatim`: emits verbatim C++ code for certain output targets.

```idl
struct S {
    @optional long opt;
    @range(min=-10, max=10) long x;
};
```

```cpp
struct S {
    omg::types::optional<int32_t> opt;
    omg::types::ranged<int32_t, -10, 10> x;
};
```

## Annotations With No Mapping Impact (selected)

- `@id`, `@autoid`, `@key`, `@must_understand`, `@unit`, `@extensibility`,
  `@final`, `@mutable`, `@appendable`, `@nested` (no mapping impact).
