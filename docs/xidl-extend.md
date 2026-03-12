# XIDL Extensions

This document describes non-OMG IDL extensions used by XIDL.

## @name

`@name("...")` renames a field for serialization.

- Applies to struct/union member fields.
- The rename value is used as the wire/property name in HTTP/JSON mappings.
- When targeting Rust, this maps to `#[serde(rename = "...")]`.
- Intended to be equivalent to `@http(rename = "...")` for HTTP generators.

Example:

```idl
module demo {
  struct User {
    uint32 id;
    @name("full_name")
    string name;
  };
};
```
