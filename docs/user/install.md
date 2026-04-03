# Install XIDL

`xidlc` is the command-line entry point for the XIDL toolchain. Most users only
need the compiler first, then add runtime crates or `xidl-build` when a target
requires them.

## Install `xidlc`

### Release installer

macOS and Linux:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://xidl.github.io/xidl/public/install.sh | sh
```

Windows PowerShell:

```powershell
iwr -useb https://xidl.github.io/xidl/public/install.ps1 | iex
```

The release installers resolve the latest stable GitHub Release and verify the
asset digest when GitHub exposes one for that artifact. macOS installers use
native `apple-darwin` archives, Linux installers prefer the static `musl`
builds, and Windows prefers the newer MSVC zip assets while still tolerating
the legacy GNU tarball on older releases.

### Cargo

```bash
cargo install xidlc
```

### Cargo Binstall

```bash
cargo binstall xidlc
```

## Package manager manifests

Formal package manager manifests are maintained in the dedicated
`xidl-packaging` repository instead of this source repository. That repository
checks the latest stable `xidl/xidl` GitHub Release every day and refreshes the
downstream package manager manifests there:

- `Formula/xidlc.rb`
- `packaging/scoop/xidlc.json`
- `packaging/winget/manifests/x/xidl/xidlc/`

The current stable release `v0.32.0` already supports Homebrew and Scoop. The
winget manifests are emitted once the published release assets include the
Windows MSVC zip archives expected by the packaging sync workflow.

## Verify the install

```bash
xidlc --help
```

The compiler uses `idlc` as its clap command name internally, but the published
binary in this repository is `xidlc`.

## Add target-specific crates when needed

Some generated outputs require companion crates at runtime.

- `rust`: generated Rust data types are plain Rust code
- `rust-jsonrpc`: pair generated code with `xidl-jsonrpc`
- `rust-axum`: pair generated code with `xidl-rust-axum`
- `openapi` and `openrpc`: emit schema files instead of a runtime crate
- `xidl-build`: use in Rust `build.rs` when you want code generation during
  compilation

## Next step

Continue with [Using xidlc](xidlc.md) for the main compiler workflow.
