set shell := ["bash", "-c"]

go_cache := env_var_or_default("GO_CACHE", "/tmp/xidl-go-cache")
go_path := env_var_or_default("GO_PATH", "/tmp/xidl-go-path")
xidlc_snapshot_version := env_var_or_default("XIDLC_SNAPSHOT_VERSION", "0.0.1")
xidlc_snapshot_hash := env_var_or_default("XIDLC_SNAPSHOT_HASH", "snapshot")
bdd_features := env_var_or_default("BDD_FEATURES", "bdd/features")

# Run all tests
test: test-rust test-go test-bdd

# Initialize typescript codec
init:
    cd ./typescript/xidl-typescript-codec && pnpm install && pnpm build
    cd ./typescript/xidl-typescript-server && pnpm install && pnpm build

# Test Rust
test-rust: init
    XIDLC_VERSION={{xidlc_snapshot_version}} XIDLC_GIT_HASH={{xidlc_snapshot_hash}} cargo test --all -F transport-all -F fmt

# Test Go
test-go: test-go-runtime

# Test BDD
test-bdd: init
    python3 -m pip install -r bdd/requirements.txt
    python3 -m behave {{bdd_features}}

# Test Go codegen
test-go-codegen:
    GO_CACHE={{go_cache}} GO_PATH={{go_path}} just --justfile golang/justfile test-go-codegen

# Test Go runtime
test-go-runtime:
    GO_CACHE={{go_cache}} GO_PATH={{go_path}} just --justfile golang/justfile test-go-runtime

# Update insta snapshots
test-update:
    INSTA_UPDATE=always XIDLC_VERSION={{xidlc_snapshot_version}} XIDLC_GIT_HASH={{xidlc_snapshot_hash}} just test-rust

# Run all coverage
test-coverage:
    cargo test -p xidl-parser -p xidl-jsonrpc -p xidl-rust-axum --all-features
    just test-xidl-parser-coverage
    just test-xidl-jsonrpc-coverage
    just test-xidl-rust-axum-coverage

# Test rust axum coverage
test-xidl-rust-axum-coverage:
    cargo tarpaulin --manifest-path crates/xidl-rust-axum/Cargo.toml --packages xidl-rust-axum --all-features --include-files "crates/xidl-rust-axum/src/*" --fail-under 80 --out Stdout

# Test parser coverage
test-xidl-parser-coverage:
    cargo tarpaulin --manifest-path xidl-parser/Cargo.toml --packages xidl-parser --all-features --include-files "xidl-parser/src/*" --exclude-files "xidl-parser/src/typed_ast/*" --fail-under 80 --out Stdout

# Test jsonrpc coverage
test-xidl-jsonrpc-coverage:
    cargo tarpaulin --manifest-path crates/xidl-jsonrpc/Cargo.toml --packages xidl-jsonrpc --all-features --include-files "crates/xidl-jsonrpc/src/*" --exclude-files "crates/xidl-jsonrpc/src/transport/io.rs" --fail-under 80 --out Stdout

# Format Jinja templates
fmt-jinja:
    cargo r -p xidlc -F cli -F fmt -- fmt -l jinja $(find xidlc -type f -name '*.j2' | sort) -i

# Build xtypes
build-xtypes:
    cargo r -p xidlc -F cli -F fmt -- gen --out-dir ./xidl-typeobject/src/ rust ./xidl-typeobject/idl/dds_xtypes_typeobject.idl

# Start docs dev server
docs-dev:
    pnpm --dir docs start

# Build docs
docs-build:
    pnpm --dir docs build
