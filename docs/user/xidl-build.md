# Using xidl-build in Rust

`xidl-build` wraps `xidlc` for Rust build scripts. Use it when you want code
generation to happen automatically during `cargo build`.

## When to use it

Use `xidl-build` when:

- your project is already Rust-first
- you want generated code under Cargo’s build pipeline
- you want `OUT_DIR` handling without calling the CLI yourself

## Add the dependency

```toml
[build-dependencies]
xidl-build = "<version>"
```

## Minimal `build.rs`

```rust
fn main() {
    xidl_build::Builder::new()
        .with_lang("rust")
        .compile(&["api/example.idl"])
        .expect("generate xidl artifacts");
}
```

If you do not set `out_dir`, the builder uses Cargo’s `OUT_DIR`.

## Common builder options

### Select a target

```rust
let builder = xidl_build::Builder::new().with_lang("rust-axum");
```

### Select an output directory

```rust
let builder = xidl_build::Builder::new().with_out_dir("src/generated");
```

### Rename single-file schema outputs

`with_output_filename` is intended for single-file schema generators:

```rust
let builder = xidl_build::Builder::new()
    .with_lang("openapi")
    .with_output_filename("city-api.json");
```

This currently works for:

- `openapi`
- `openrpc`

### Control client/server generation

```rust
let builder = xidl_build::Builder::new()
    .with_lang("rust-axum")
    .with_client(true)
    .with_server(false);
```

## Practical example

```rust
fn main() {
    println!("cargo:rerun-if-changed=api/city.idl");

    xidl_build::Builder::new()
        .with_lang("openapi")
        .with_out_dir("generated")
        .with_output_filename("city.json")
        .compile(&["api/city.idl"])
        .expect("generate openapi");
}
```

## Output behavior

- `compile` accepts a slice of input paths.
- generation runs through the same internal driver used by `xidlc`
- relative output filenames are resolved under `out_dir`
- `with_output_filename` is rejected for targets that produce multiple files

## Related material

- [Using xidlc](xidlc.md)
- [Targets Reference](../reference/targets.md)
