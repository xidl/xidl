# IDL4 to Rust JSON-RPC mapping

This document describes xidlc’s Rust JSON-RPC generator. It extends the Rust
mapping rules in `idl2rust.md`, but only targets interfaces and modules. Other
IDL definitions are ignored in JSON-RPC output.

## General

- Based on Rust mapping with JSON-RPC-specific overrides.
- Only `module` and `interface` definitions are emitted.
- IDL modules map to Rust modules of the same name.
- Method names use the raw IDL identifier for the JSON-RPC method name, and the
  Rust-escaped identifier for the Rust trait method.
- Fully-qualified JSON-RPC method name is:
  `module_path.join(".") + "." + interface + "." + method`.

## Type Mapping Overrides

- `any`, `object`, and `valuebase` map to `serde_json::Value`.
- `sequence<T>` maps to `Vec<T>`.
- `map<K, V>` maps to `BTreeMap<K, V>`.
- `string` / `wstring` map to `String`.
- `fixed` maps to `f64`.
- Other scalar and enum mappings follow `idl2rust.md`.

All interface parameters are passed by value in JSON-RPC (no `&` or `&mut`),
and `in/out/inout` attributes are currently ignored.

## Interfaces

Each IDL interface produces:

- A Rust trait with JSON-RPC error handling:
  `fn method(...) -> Result<Ret, xidl_jsonrpc::Error>`.
- A server wrapper `InterfaceServer<T>` implementing `xidl_jsonrpc::Handler`.
- A client wrapper `InterfaceClient<R, W>` that implements the trait and makes
  JSON-RPC calls.
- For each method, a `Params` struct with `#[derive(Serialize, Deserialize)]`
  that bundles method arguments.

Attributes map to RPC methods:

- `readonly attribute foo` -> RPC method `foo`.
- `attribute foo` -> RPC methods `foo` and `set_foo`.
- `readonly` attributes with `raises` are currently skipped.

### Example

```idl
module math {
    interface Calc {
        long add(in long a, in long b);
        readonly attribute long version;
        attribute string name;
    };
}
```

```rust
pub trait Calc {
    fn add(&self, a: i32, b: i32) -> Result<i32, xidl_jsonrpc::Error>;
    fn version(&self) -> Result<i32, xidl_jsonrpc::Error>;
    fn name(&self) -> Result<String, xidl_jsonrpc::Error>;
    fn set_name(&self, value: String) -> Result<(), xidl_jsonrpc::Error>;
}

#[derive(Serialize, Deserialize)]
struct CalcAddParams {
    a: i32,
    b: i32,
}

pub struct CalcServer<T> {
    inner: T,
}

impl<T> xidl_jsonrpc::Handler for CalcServer<T>
where
    T: Calc,
{
    fn handle(&self, method: &str, params: Value) -> Result<Value, xidl_jsonrpc::Error> {
        match method {
            "math.Calc.add" => { /* ... */ }
            "math.Calc.version" => { /* ... */ }
            "math.Calc.name" => { /* ... */ }
            "math.Calc.set_name" => { /* ... */ }
            _ => Err(xidl_jsonrpc::Error::method_not_found(method)),
        }
    }
}
```

## Notes and Limitations

- JSON-RPC output only includes interfaces; structs, enums, unions, etc. are
  expected to be available from the Rust generator output or other crates.
- `raises` clauses are ignored and do not affect signatures.
- `@derive(...)` annotations are not used in JSON-RPC output (only internal
  params structs derive `Serialize`/`Deserialize`).
