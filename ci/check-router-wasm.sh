#!/usr/bin/env bash
# Compile a generated service with mixed authentication using only router support.
set -euo pipefail
cd "$(dirname "$0")/.."
probe_dir=$(mktemp -d)
trap 'rm -rf "$probe_dir"' EXIT
mkdir -p "$probe_dir/src"
cargo run --locked -p xidlc --features cli,fmt -- gen --out-dir "$probe_dir/src" \
  rust-axum --server xidlc/tests/shared/http_security_inheritance.idl
printf 'pub mod http_security_inheritance;\n' > "$probe_dir/src/lib.rs"
python3 - "$PWD/crates/xidl-rust-axum" "$probe_dir/Cargo.toml" <<'PYTHON'
import json
import pathlib
import sys
pathlib.Path(sys.argv[2]).write_text("""[package]
name = "xidl-router-wasm-check"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
serde = { version = "1", features = ["derive"] }
async-trait = "0.1"
xidl-rust-axum = { path = """ + json.dumps(sys.argv[1]) + """, default-features = false, features = ["router"] }
""")
PYTHON
cargo check --manifest-path "$probe_dir/Cargo.toml" --target wasm32-unknown-unknown
