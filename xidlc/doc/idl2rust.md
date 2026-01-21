# IDL4 to Rust mapping

## Core Data Types

### Modules

```idl
module my_math {

}
```

Would map to the following Rust code:

```rust
mod my_math {

}
```

### Constants

```idl
module my_math {
    const string my_string = "My String Value";
    const double PI = 3.141592;
}
```

become:

```rust
mod my_math {
    const my_string: &'static str = "My String Value";
    cosnt PI: f64 = 3.141592;
}
```

Don't support wstring.

## Data Types

### Basic Types

#### Integer Types

| IDL Type           | Rust Type |
| ------------------ | --------- |
| short              | i16       |
| unsigned short     | u16       |
| long               | i32       |
| unsigned long      | u32       |
| long long          | i64       |
| unsigned long long | u64       |

#### Floating Point Types

| IDL Type    | Rust Type |
| ----------- | --------- |
| float       | f32       |
| double      | f64       |
| long double | f64       |

#### char type

The IDL `char` type shall be mapped to the Rust `char` type.

#### wchar type

DON'T SUPPORT YET

#### Boolean type

The IDL `boolean` type shall be mapped to the Rust `bool` type. the IDL
constants `TRUE` and `FALSE` shall be mapped to the Rust constants `true` and
`false`.

#### octet type

The IDL `octet` type shall be mapped to the Rust `u8` type.

### Template types

TODO

### Strings

The IDL `string` type shall be mapped to the Rust `String` type.

The IDL `string<N>` DON'T SUPPORT YET.

### Wstrings

DON'T SUPPORT YET.

### Fixed Type

DON'T SUPPORT YET.

### Constructed types

#### Structures

An IDL struct shall be mapped to a Rust struct with the same name. The mapped
struct shall provide:

- A default constructor that initializes all built-in data types to their
  default value as specified in Clause 7.2.4.1, enumerators to their first
  value, and the rest of members using their default constructor. constructed
  type with strong type safety.
- A set of comparison operators, including at least "equal to" and "not equal
  to."
- A `Debug` trait would be implemented.

For Example:

```idl
struct MyStruct {
    long a_long;
    short a_short;
};
```

would map to the following Rust struct:

```rust
#[derive(Default, PartialEq, Eq, Clone, Debug)]
pub struct MyStruct {
    pub a_long: i32,
    pub a_short: i16,
}
```

#### Unions

An IDL union shall be mapped to a Rust struct with the same name. The structure
shall provide the same constructors, destructors, and operators for mapped
structures defined in Clause 7.2.4.3.1.

Moreover, the mapped class shall provide:

- A public accessor constant method named _d() that returns the value of the
  discriminator, with the following signature:

  ```rust
  fn _d(&self) <DiscriminatorType>;
  ```

- A public modifier method named _d() that sets the value of the discriminator
  with the following signature:
  ```rust
  fn _d(&mut self, value: <DiscriminatorType>);
  ```
  Setting the discriminator to an invalid value (e.g., to a value that changes
  the union member that is currently selected) may result in an error
- For each union member:
  - A public accessor method with the name of the union member:
    ```rust
    fn <member_name>(&self) -> &<member_type>;
    fn <member_name>_mut(&mut self) -> &mut <member_type>;
    ```
    Accessing an invalid union member may result in an undefined error.
- If the union has a default case, the default constructor shall initialize the
  discriminator, and the selected member field following the initialization
  rules described in Clause 7.2.4.3.1. If it does not, the default constructor
  shall initialize the union to the first discriminant value specified in the
  IDL definition
- If the IDL union definition has an implicit default member (i.e., if the union
  does not have a default case and not all permissive discriminator values are
  listed), the class shall provide a method named _default() that explicitly
  sets the discriminator value to a legal default value. void _default();

For example, the IDL `union` declaration below:

```idl
union AUnion switch (octet) {
    case 1:
        long a_long;
    case 2:
    case 3:
        short a_short;
    case 4:
        AStruct a_struct;
    default:
        octet a_byte_default;
    };
```

would map to the following Rust code:

```rust
pub struct AUnion {
}

impl Default for AUnion {
    fn default() -> Self {
        //
    }
}

impl AUnion {
    pub fn _d(&self);
    pub fn a_long(&);
}
```
