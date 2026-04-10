GO_CACHE ?= /tmp/xidl-go-cache
GO_PATH ?= /tmp/xidl-go-path

.PHONY: test test-rust test-go test-go-codegen test-go-runtime test-update test-coverage test-xidl-parser-coverage test-xidl-jsonrpc-coverage test-xidl-rust-axum-coverage build-xtypes docs-dev docs-build

test: test-rust test-go test-coverage

test-rust:
	cargo test --all -F transport-all -F fmt

test-go: test-go-codegen test-go-runtime

test-go-codegen:
	$(MAKE) -C golang test-go-codegen GO_CACHE=$(GO_CACHE) GO_PATH=$(GO_PATH)

test-go-runtime:
	$(MAKE) -C golang test-go-runtime GO_CACHE=$(GO_CACHE) GO_PATH=$(GO_PATH)

test-update:
	INSTA_UPDATE=always make test-rust

test-coverage:
	cargo test -p xidl-parser -p xidl-rust-axum
	$(MAKE) test-xidl-parser-coverage
	$(MAKE) test-xidl-jsonrpc-coverage
	$(MAKE) test-xidl-rust-axum-coverage

test-xidl-rust-axum-coverage:
	cargo tarpaulin --manifest-path xidl-rust-axum/Cargo.toml --packages xidl-rust-axum --include-files "xidl-rust-axum/src/*" --fail-under 95 --out Stdout

test-xidl-parser-coverage:
	cargo tarpaulin --manifest-path xidl-parser/Cargo.toml --packages xidl-parser --include-files "xidl-parser/src/*" --exclude-files "xidl-parser/src/typed_ast/*" --fail-under 95 --out Stdout

test-xidl-jsonrpc-coverage:
	cargo tarpaulin --manifest-path xidl-jsonrpc/Cargo.toml --packages xidl-jsonrpc --include-files "xidl-jsonrpc/src/*" --fail-under 95 --out Stdout

build-xtypes:
	cargo r -p xidlc -- --lang rust --out-dir ./xidl-typeobject/src/ ./xidl-typeobject/idl/dds-xtypes_typeobject.idl

docs-dev:
	pnpm --dir docs start

docs-build:
	pnpm --dir docs build
